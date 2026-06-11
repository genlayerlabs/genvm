"""
Integration test plugin for ya-test-runner.

This plugin collects and runs integration tests from .jsonnet files in tests/cases/.
It uses the same MockHost/base_host infrastructure as the old runner.
"""

import base64
import gzip
import json
import os
import pickle
import re
import shutil
import sys
import typing
from dataclasses import dataclass
from pathlib import Path

import ya_test_runner
from ya_test_runner.exec.command import Command, RunMode
from ya_test_runner.test import CONST_PASSED
import ya_test_runner_plugins.genvm as genvm

import difflib
import itertools
import io

# Get the local context
local_ctx = ya_test_runner.stage.configuration.current_context()

# Load build info
build_info = json.loads(
	local_ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
)

BUILD_DIR = Path(build_info['build_dir'])
TARGET_DIR = Path(build_info['rust_target_dir'])

local_ctx.run_parser.add_argument(
	'--ignore-hash',
	action='store_true',
	default=False,
	help='Ignore .hash files entirely (skip hash comparison and do not write missing ones)',
)


def _is_ignore_hash_enabled() -> bool:
	return '--ignore-hash' in sys.argv


# Set up paths for importing plugin modules
TESTS_DIR = local_ctx.shared.root_dir.joinpath('tests')
CASES_DIR = TESTS_DIR.joinpath('cases')
TEMPLATES_DIR = TESTS_DIR.joinpath('templates')

import genlayer.calldata as gvm_calldata
from genlayer.types import Address
from gvm_extra.mock_host import MockHost as MockHost, MockStorage as MockStorage
import origin.base_host as base_host
import origin.fees as fees
import origin.logger as origin_logger
import origin.public_abi as public_abi

# Default environment for tests
default_env = {
	k: v
	for k, v in os.environ.items()
	if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
}

# `prepare` scripts run `cargo build`; the host `rustc` used for build
# scripts/proc-macros needs libz etc. on LD_LIBRARY_PATH. Fold in
# CARGO_LD_LIBRARY_PATH the same way cargo.py does for its own commands.
_cargo_ld_library_path = os.environ.get('CARGO_LD_LIBRARY_PATH', None)
_base_ld_library_path = default_env.get('LD_LIBRARY_PATH', None)
_new_ld_library_path = ':'.join(
	filter(None, [_cargo_ld_library_path, _base_ld_library_path])
)
if _new_ld_library_path:
	default_env['LD_LIBRARY_PATH'] = _new_ld_library_path


class _SavedLog(typing.TypedDict):
	level: str
	msg: str
	kwargs: dict


def _make_log_adapter(formatter: ya_test_runner.Formatter) -> 'origin_logger.Logger':
	class _FormatterLoggerAdapter(origin_logger.Logger):
		"""Adapts ya_test_runner.formatter.Formatter to base_host.Logger interface."""

		saved_logs: list[_SavedLog]

		def __init__(self, formatter: ya_test_runner.Formatter):
			self._formatter = formatter

			self.saved_logs = []

		def log(self, level: str, msg: str, **kwargs) -> None:
			fmt_level = ya_test_runner.Formatter.Level.from_str(level)
			self._formatter.log(fmt_level, msg, **kwargs)
			self.saved_logs.append(
				{
					'level': level,
					'msg': msg,
					'kwargs': kwargs,
				}
			)

	global make_adapter
	make_adapter = _FormatterLoggerAdapter

	return make_adapter(formatter)


def _unfold_conf(x: typing.Any, vars: dict[str, str]) -> typing.Any:
	"""Recursively substitute variables in configuration."""
	if isinstance(x, str):
		return re.sub(r'\$\{[a-zA-Z\-_]+\}', lambda m: vars[m.group()[2:-1]], x)
	if isinstance(x, list):
		return [_unfold_conf(item, vars) for item in x]
	if isinstance(x, dict):
		return {k: _unfold_conf(v, vars) for k, v in x.items()}
	return x


def _flatten_tree(
	entries: list[dict],
) -> list[tuple[str, str | None, dict]]:
	"""
	DFS-traverse the step tree and return (tree_path, depends_on_tree_path, step_conf) tuples.

	The depends_on field is computed in the jsonnet template (util.jsonnet expandModes).
	A null depends_on means the step depends on /prepare.
	"""
	result: list[tuple[str, str | None, dict]] = []
	for entry in entries:
		step_conf = {k: v for k, v in entry.items() if k != 'next'}
		tree_path = step_conf['tree_path']
		depends_on = step_conf.get('depends_on')

		result.append((tree_path, depends_on, step_conf))
		if 'next' in entry:
			result.extend(_flatten_tree(entry['next']))
	return result


_TOP_LEVEL_METADATA_KEYS = frozenset({'entry', 'tags'})


@dataclass
class IntegrationPrepareCase(ya_test_runner.test.Case):
	"""Test case for the preparation/setup phase of an integration test."""

	description: ya_test_runner.test.Description
	jsonnet_path: Path
	top_level_conf: dict
	tmp_dir: Path

	hidden: bool = True

	async def into_steps(self) -> list[ya_test_runner.exec.step.Step]:
		return [IntegrationSetupStep(self)]


@dataclass
class IntegrationSingleCase(ya_test_runner.test.Case):
	"""Test case for a single step of an integration test."""

	description: ya_test_runner.test.Description
	jsonnet_path: Path
	manager_service: ya_test_runner.stage.collection.Service
	tree_path: str
	parent_tree_path: str | None
	single_conf: dict
	total_steps: int
	tmp_dir: Path
	max_attempts: int
	is_benchmark: bool = False

	async def into_steps(self) -> list[ya_test_runner.exec.step.Step]:
		step = IntegrationSingleStep(self)
		if self.is_benchmark:
			steps: list[ya_test_runner.exec.step.Step] = []
			for i in range(10):
				steps.append(CheckInterruptedStep())
				steps.append(ya_test_runner.test.BenchMeasureStep())
				steps.append(step)
			steps.append(
				ya_test_runner.test.BenchCollectStep(
					local_ctx.shared.printer, test_name=self.description.name
				)
			)
			return steps
		return [step]


class CheckInterruptedStep(ya_test_runner.exec.step.Python):
	def to_str(self):
		return '<check interrupted>'

	async def run(self, previous_results: list[typing.Any]):
		if local_ctx.shared.is_interrupted:
			raise ya_test_runner.test.FinishedEarlyException(
				ya_test_runner.test.Result(
					passed=False,
					context={'reason': 'interrupted'},
					elapsed_seconds=0,
				)
			)


class IntegrationSkipStep(ya_test_runner.exec.step.Python):
	"""Returns a skipped result for tests with .skip file."""

	def __init__(self, test_name: str):
		self._test_name = test_name

	def to_str(self) -> str:
		return f'<skip: {self._test_name}>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		local_ctx.shared.logger.warning(
			'Test skipped',
			test_name=self._test_name,
		)
		return ya_test_runner.test.Result(
			passed=True,
			context={'skipped': True},
			elapsed_seconds=0,
		)


class IntegrationSetupStep(ya_test_runner.exec.step.Python):
	"""Sets up the test environment: temp dir, prepare script, base storage."""

	def __init__(self, case: IntegrationPrepareCase):
		self._case = case

	def to_str(self) -> str:
		return f'<setup: {self._case.jsonnet_path.name}>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		tmp_dir = self._case.tmp_dir
		top_level_conf = self._case.top_level_conf

		# Set up temp directory
		shutil.rmtree(tmp_dir, ignore_errors=True)
		tmp_dir.mkdir(exist_ok=True, parents=True)

		# Run preparation if needed
		if 'prepare' in top_level_conf:
			cmd = Command(
				args=[sys.executable, top_level_conf['prepare']],
				cwd=self._case.jsonnet_path.parent,
				env=default_env,
			)
			result = await cmd.run(local_ctx.shared, mode=RunMode.SILENT)
			if result.exit_code != 0:
				raise ya_test_runner.test.FinishedEarlyException(
					result=ya_test_runner.test.Result(
						passed=False,
						context={
							'reason': 'prepare script failed',
							'exit_code': result.exit_code,
							'stdout': result.stdout,
							'stderr': result.stderr,
							'log': result.stderr,
						},
						elapsed_seconds=0,
					)
				)

		# Set up base storage
		base_mock_storage = MockStorage()
		if storage_json := top_level_conf.get('storage_json'):
			storage_b64 = json.loads(Path(storage_json).read_text())
			base_mock_storage._storages = {
				Address(a): {
					base64.b64decode(k): bytearray(base64.b64decode(v)) for k, v in kv.items()
				}
				for a, kv in storage_b64.items()
			}

		empty_storage = tmp_dir.joinpath('empty-storage.pickle')
		with open(empty_storage, 'wb') as f:
			pickle.dump(base_mock_storage, f)

		return ya_test_runner.test.Result(
			passed=True,
			context={},
			elapsed_seconds=0,
		)


FAKE_TX_ID = '0x' + '00' * 32
FAKE_NODE_ADDRESS = '0xE840F4456F4cD28C4f54d0F8AfA2C0DBf43e4d29'
FAKE_NODE_PRIVATE_KEY = (
	'81bd0b16ba7f9a06ca3e0e54796018b4792dbc406a93421bb8789af2dd139809'
)
FAKE_NODE_PUBLIC_KEY = '6478c39d71a8e469a2dfc5f467ab48e449012308228ab81aa2341107ea7bb3324ab8d4169d49f4705a35b7271475f6d81e210aa2ff35fea4d74d83d25ec6599c'
SIGNER_URL = 'https://test-server.genlayer.com/genvm/sign'


def _get_diffs[T](exp: T, got: T, dump: typing.Callable[[T], str]) -> dict | None:
	if exp == got:
		return None
	exp_txt = dump(exp)
	got_txt = dump(got)
	diff = difflib.unified_diff(
		exp_txt.splitlines(keepends=False),
		got_txt.splitlines(keepends=False),
	)

	return {
		'exp': exp,
		'got': got,
		'diff': '\n'.join(itertools.islice(diff, 5)),
	}


def _calldata_to_fancy_str(calldata: bytes) -> str:
	cd = gvm_calldata.decode(calldata)
	buf = io.StringIO()
	ya_test_runner.formatter.TextFormatter(
		ya_test_runner.formatter.NoLockTextIO(buf)
	).dump(ya_test_runner.formatter.Formatter.Level.ERROR, 'calldata', calldata=cd)
	return buf.getvalue()


class Context(base_host.Context):
	logger: base_host.Logger

	def __init__(self, logger: base_host.Logger):
		self.logger = logger

	def on_genvm_success(self): ...
	def on_genvm_failure(self): ...

	def add_stat(self, key: str, value: typing.Any):
		self.logger.debug('stat', key=key, val=value)

	def get_timeout(
		self, action: base_host.TimeoutAction, type: base_host.TimeoutType
	) -> float | None:
		if type == base_host.TimeoutType.CONNECT_S:
			return 5.0
		return None


class IntegrationSingleStep(ya_test_runner.exec.step.Python):
	"""Executes a single step of an integration test."""

	def __init__(self, case: IntegrationSingleCase):
		self._test_case = case
		self._tree_path = case.tree_path
		self._parent_tree_path = case.parent_tree_path
		self._single_conf = case.single_conf
		self._total_steps = case.total_steps
		self._tmp_dir = case.tmp_dir
		self._max_attempts = case.max_attempts

	def to_str(self) -> str:
		return f'<step {self._tree_path}: {self._test_case.jsonnet_path.name}>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		empty_storage = self._tmp_dir.joinpath('empty-storage.pickle')

		for attempt in range(self._max_attempts):
			sub_logger = _make_log_adapter(local_ctx.shared.logger)
			result = await self._run_single_step(empty_storage, sub_logger)
			if result['passed']:
				return ya_test_runner.test.Result(
					passed=True,
					context=result.get('context', {}),
					elapsed_seconds=0,
				)

			if local_ctx.shared.is_interrupted or attempt + 1 >= self._max_attempts:
				# Raise FinishedEarlyException to stop subsequent steps
				context = result.get('context', {})
				context['logs'] = sub_logger.saved_logs
				raise ya_test_runner.test.FinishedEarlyException(
					result=ya_test_runner.test.Result(
						passed=False,
						context=context,
						elapsed_seconds=0,
					)
				)

			local_ctx.shared.logger.warning(
				f'Unstable test failed',
				attempt=attempt + 1,
				max_attempts=self._max_attempts,
				test_name=str(self._test_case.description.name),
				tree_path=self._tree_path,
				context=result.get('context', {}),
			)

		# Should not reach here
		raise ya_test_runner.test.FinishedEarlyException(
			result=ya_test_runner.test.Result(passed=False, context={}, elapsed_seconds=0)
		)

	async def _run_single_step(
		self, empty_storage: Path, logger: 'origin_logger.Logger'
	) -> dict:
		single_conf = pickle.loads(pickle.dumps(self._single_conf))  # Deep copy
		jsonnet_path = self._test_case.jsonnet_path
		tree_path = self._tree_path
		parent_tree_path = self._parent_tree_path

		my_tmp_dir = self._tmp_dir.joinpath(tree_path)
		suff = f'.{tree_path}'

		my_tmp_dir.mkdir(exist_ok=True, parents=True)

		# Set up storage paths
		if parent_tree_path is None:
			pre_storage = empty_storage
		else:
			pre_storage = self._tmp_dir.joinpath(parent_tree_path + '_storage.pickle')
		post_storage = my_tmp_dir.parent.joinpath(my_tmp_dir.name[:-1] + '_storage.pickle')

		config_str = json.dumps(single_conf, indent=2)
		my_tmp_dir.joinpath('config.json').write_text(config_str)

		# Prepare calldata
		calldata_eval_vars = {}
		calldata_eval_vars['Address'] = gvm_calldata.Address
		calldata_eval_vars['true'] = True
		calldata_eval_vars['false'] = False

		calldata_bytes = gvm_calldata.encode(
			eval(
				single_conf['calldata'],
				calldata_eval_vars,
				single_conf.get('vars', {}).copy(),
			)
		)

		# Process code file
		code_path = single_conf.get('code')
		code = None
		if code_path is not None:
			if code_path.endswith('.wat'):
				out_path = my_tmp_dir.joinpath(Path(code_path).with_suffix('.wasm').name)
				cmd = Command(
					args=[
						'wat2wasm',
						'--enable-tail-call',
						'--enable-annotations',
						'-o',
						str(out_path),
						code_path,
					],
					cwd=my_tmp_dir,
					env=default_env,
				)
				result = await cmd.run(local_ctx.shared, mode=RunMode.SILENT)
				if result.exit_code != 0:
					return {
						'passed': False,
						'context': {
							'reason': 'wat2wasm failed',
							'tree_path': tree_path,
							'exit_code': result.exit_code,
							'stdout': result.stdout,
							'stderr': result.stderr,
						},
					}
				code_path = str(out_path)
			code = Path(code_path).read_bytes()

		# Process message addresses
		single_conf['message']['contract_address'] = Address(
			single_conf['message']['contract_address']
		)
		single_conf['message']['sender_address'] = Address(
			single_conf['message']['sender_address']
		)
		single_conf['message']['origin_address'] = Address(
			single_conf['message']['origin_address']
		)

		path_to_which_leader_puts_result = my_tmp_dir.parent.joinpath(
			my_tmp_dir.name[:-1] + '_leader_nondet.pickle'
		)

		# Set up paths
		rel_path = jsonnet_path.relative_to(CASES_DIR)
		mock_sock_path = Path('/tmp', 'genvm-test', rel_path.with_suffix(f'.sock{suff}'))
		mock_sock_path.parent.mkdir(exist_ok=True, parents=True)

		is_leader = single_conf.get('mode') == 'l'

		# Load leader nondet results if available (for v/s modes)
		if is_leader:
			leader_nondet = None
		else:
			leader_nondet = single_conf.get('leader_nondet', None)
			if leader_nondet is None:
				with open(path_to_which_leader_puts_result, 'rb') as f:
					leader_nondet = pickle.load(f)
			if leader_nondet is not None:
				encoded_nondet = []
				for res in leader_nondet:
					if isinstance(res, (bytes, bytearray, memoryview)):
						encoded_nondet.append(bytes(res))
					elif res['kind'] == 'return':
						encoded_nondet.append(
							bytes([public_abi.ResultCode.RETURN]) + gvm_calldata.encode(res['value'])
						)
					elif res['kind'] == 'user_error':
						if isinstance(res['value'], str):
							encoded_nondet.append(
								bytes([public_abi.ResultCode.USER_ERROR]) + res['value'].encode('utf-8')
							)
						else:
							encoded_nondet.append(
								bytes([public_abi.ResultCode.USER_ERROR])
								+ b'\x00\x00\x00\x00'
								+ gvm_calldata.encode(res['value'])
							)
					elif res['kind'] == 'vm_error':
						encoded_nondet.append(
							bytes([public_abi.ResultCode.VM_ERROR]) + res['value'].encode('utf-8')
						)
					elif res['kind'] == 'raw':
						encoded_nondet.append(bytes(res['value']))
					else:
						raise ValueError(f'unknown leader_nondet kind: {res["kind"]}')
				leader_nondet = encoded_nondet

		# Create mock host
		running_address = single_conf['message']['contract_address']
		host = MockHost(
			path=str(mock_sock_path),
			storage_path_post=post_storage,
			storage_path_pre=pre_storage,
			balances={Address(k): v for k, v in single_conf.get('balances', {}).items()},
			running_address=running_address,
		)

		host.balances.setdefault(running_address, 0)
		host.balances[running_address] += single_conf['message'].get('value', 0)

		# Get manager URI from the service
		manager_svc = self._test_case.manager_service
		port = manager_svc.meta['port']
		manager_uri = f'http://localhost:{port}'

		# Run the test
		with host as mock_host:
			try:
				host_data = json.dumps(
					{
						'node_address': FAKE_NODE_ADDRESS,
						'tx_id': FAKE_TX_ID,
						'signerUrl': SIGNER_URL,
					}
				)
				request_extra = {}
				if 'stable' in self._test_case.description.tags:
					request_extra['no_modules'] = True
				dflt_bucket = 2**200
				bucket_totals: list[int] = single_conf.get(
					'bucket_totals', [dflt_bucket, dflt_bucket]
				)

				default_message_fee_allocation = [
					fees.DEFAULT_EXTERNAL_MESSAGE_ALLOC,
					fees.DEFAULT_INTERNAL_FIN_MESSAGE_ALLOC,
					fees.DEFAULT_INTERNAL_ACC_MESSAGE_ALLOC,
				]

				message_fee_allocation: list[fees.MessageAllocationNode] = single_conf.get(
					'message_fee_allocation', default_message_fee_allocation
				)

				if not single_conf['message'].get('is_init', False):
					code = None
				mode = single_conf.get('mode', 'l')
				ctx = Context(logger)
				res = await base_host.run_genvm(
					mock_host,
					manager_uri=manager_uri,
					message=single_conf['message'],
					timeout=single_conf.get('deadline', 10 * 60),
					capture_output=True,
					is_sync=(mode == 's'),
					host_data=host_data,
					ctx=ctx,
					host='unix://' + mock_host.path,
					extra_args=['--debug-mode'],
					code=code,
					calldata=calldata_bytes,
					bucket_totals=bucket_totals,
					gas_data=single_conf.get('gas_data'),
					leader_nondet_results=leader_nondet,
					message_fee_allocation=message_fee_allocation,
					request_extra=request_extra,
				)
				stdout_raw = res.stdout
				return_part = ''
				if res.result_kind == public_abi.ResultCode.RETURN:
					return_part += (
						f'executed with `Return({gvm_calldata.to_str(res.result_data)})`\n'
					)
				elif res.result_kind == public_abi.ResultCode.VM_ERROR:
					return_part += f'executed with `VMError("{res.result_data}")`\n'
				elif res.result_kind == public_abi.ResultCode.USER_ERROR:
					if isinstance(res.result_data, str):
						return_part += f'executed with `UserError("{res.result_data}")`\n'
					else:
						return_part += (
							f'executed with `UserError({gvm_calldata.to_str(res.result_data)})`\n'
						)
				nondet_part = ''
				if mock_host.nondet_disagreement_call_no is not None:
					nondet_part = (
						f'nondet disagreement: {mock_host.nondet_disagreement_call_no}\n'
					)
				res.stdout = stdout_raw + return_part + nondet_part

				for k, v in res.result_storage_changes:
					assert len(k) == 36
					index = int.from_bytes(k[32:], byteorder='big')
					index *= 32
					if index >= 4096:
						logger.warning(
							'suspicious storage writing', index=index, key=k.hex(), value=v.hex()
						)
					mock_host.storage.write(
						mock_host.running_address,
						k[:32],
						index,
						v,
					)
			except Exception as e:
				return {
					'passed': False,
					'context': {
						'exception': e,
						'tree_path': tree_path,
					},
				}

		# Save leader nondet results for v/s modes
		if is_leader:
			nondet_list = res.result_nondet_results

			with open(path_to_which_leader_puts_result, 'wb') as f:
				pickle.dump(nondet_list, f)

		# Save outputs
		my_tmp_dir.joinpath('stdout.txt').write_text(res.stdout)
		my_tmp_dir.joinpath('stderr.txt').write_text(res.stderr)
		with gzip.open(
			my_tmp_dir.joinpath('genvm.log.gz'), 'wt', compresslevel=5
		) as log_file:
			for log_line in res.genvm_log:
				json.dump(log_line, log_file)
				log_file.write('\n')

		# Save RunHostAndProgramRes pickle
		result_path = Path(single_conf['result_path'])
		result_path.parent.mkdir(exist_ok=True, parents=True)
		with open(result_path, 'wb') as f:
			pickle.dump(res, f)

		if res.result_kind == public_abi.ResultCode.INTERNAL_ERROR:
			return {
				'passed': False,
				'context': {
					'reason': 'internal error',
					'result_data': res.result_data,
					'stderr': res.stderr,
					'stdout': res.stdout,
					'genvm_log': res.genvm_log,
				},
			}

		result_events: list[list[bytes]] = []

		messages_content = []
		for em in res.result_emissions:
			if em['type'] == 'EmitEvent':
				tem = typing.cast(base_host.EmitEventInner, em)
				blob = gvm_calldata.encode(tem['blob'])
				result_events.append([*tem['topics'], blob])
			messages_content.append(gvm_calldata.to_str(em))
			messages_content.append('\n')

		# Write hash file (calldata-encoded deterministic result fields)
		hash_data = gvm_calldata.encode(
			[
				int(res.result_kind),
				res.result_data,
				res.result_fingerprint,
				res.result_storage_changes,
				result_events,
			]
		)

		semantics_parts = {
			'stdout': stdout_raw,
			'return': return_part,
			'nondet': nondet_part,
			'kind': res.result_kind.name + '\n',
			'messages': ''.join(messages_content),
		}
		semantics_components = single_conf['expected_semantics_components']
		if semantics_components:
			semantics = ''.join(semantics_parts[c] for c in semantics_components)

			semantics_path = my_tmp_dir.joinpath('semantics.txt')
			semantics_path.write_text(semantics)

			exp_semantics_path = Path(single_conf['expected_semantics_path'])
			if exp_semantics_path.exists():
				exp_text = exp_semantics_path.read_text()
				diff = _get_diffs(exp_text, semantics, lambda x: x)
				if diff is not None:
					return {
						'passed': False,
						'context': {
							'reason': 'semantics mismatch',
							'expected_path': str(exp_semantics_path),
							'got_path': str(semantics_path),
							'semantics': semantics,
							'stderr': res.stderr,
							'genvm_log': res.genvm_log,
							**diff,
						},
					}
			else:
				# Create expected output file
				exp_semantics_path.write_text(semantics)

		my_tmp_dir.joinpath('hash').write_bytes(base64.b64encode(hash_data))

		expected_hash_path = single_conf['expected_hash_path']

		check_expected_hash = expected_hash_path is not None
		if (
			res.result_kind == public_abi.ResultCode.VM_ERROR and res.result_data == 'timeout'
		):
			check_expected_hash = False  # Don't check hash for timeouts, as they can be flaky
		if _is_ignore_hash_enabled():
			check_expected_hash = False

		if check_expected_hash:
			expected_hash_path = Path(expected_hash_path)

			if expected_hash_path.exists():
				original_hash = base64.b64decode(expected_hash_path.read_text().strip())

				diff = _get_diffs(original_hash, hash_data, _calldata_to_fancy_str)

				if diff is not None:
					return {
						'passed': False,
						'context': {
							'reason': 'hash mismatch',
							'hash_path': str(expected_hash_path),
							'tree_path': tree_path,
							**diff,
						},
					}
			else:
				expected_hash_path.parent.mkdir(exist_ok=True, parents=True)
				expected_hash_path.write_text(
					base64.b64encode(hash_data).decode('ascii') + '\n'
				)

		return {'passed': True, 'context': {'execution_time': res.execution_time}}


def _test_needs_webdriver(jsonnet_path: Path) -> bool:
	"""Check if a test needs webdriver based on its content."""
	content = jsonnet_path.read_text()
	# Web tests typically involve screenshots or webpage interactions
	return 'screenshot' in content.lower() or 'get_webpage' in content.lower()


def integration_test(
	ctx: ya_test_runner.stage.collection.Context,
	*,
	manager_service: ya_test_runner.stage.collection.Service,
	modules_service: ya_test_runner.stage.collection.Service,
	webdriver_service: ya_test_runner.stage.collection.Service,
) -> None:
	"""
	Collect integration tests from tests/cases/ directory.

	Each jsonnet file produces multiple test cases:
		- <name>/prepare  — setup (temp dir, prepare script, base storage)
		- <name>/<tree_path><mode> — one per step (e.g. /0l, /0v, /0s)

	Steps depend on /prepare, and child steps depend on their parent step.
	"""

	jsonnet_files = list(CASES_DIR.glob('**/*.jsonnet'))
	jsonnet_files.sort()

	# pre-import
	import _jsonnet

	from concurrent.futures import ThreadPoolExecutor, as_completed

	with ThreadPoolExecutor(thread_name_prefix='integration-test-collector') as executor:
		futures = {
			executor.submit(
				_single_integration_test,
				ctx,
				jsonnet_file,
				manager_service=manager_service,
				modules_service=modules_service,
				webdriver_service=webdriver_service,
			): jsonnet_file
			for jsonnet_file in jsonnet_files
		}
		for future in as_completed(futures):
			future.result()


def _single_integration_test(
	ctx: ya_test_runner.stage.collection.Context,
	jsonnet_file: Path,
	*,
	manager_service: ya_test_runner.stage.collection.Service,
	modules_service: ya_test_runner.stage.collection.Service,
	webdriver_service: ya_test_runner.stage.collection.Service,
) -> None:
	import _jsonnet

	rel_path = jsonnet_file.relative_to(CASES_DIR)

	# Determine stability tag from path
	tags: set[str] = {'integration'}
	stability_tag = rel_path.parts[0] if rel_path.parts else 'unknown'
	if stability_tag in ('stable', 'unstable', 'semi-stable'):
		tags.add(stability_tag)
	elif stability_tag.startswith('_'):
		tags.add('stable')
		stability_tag = 'stable'

	needed_services: set[ya_test_runner.stage.collection.Service] = {manager_service}

	if 'stable' not in tags:
		needed_services.add(modules_service)
		needed_services.add(webdriver_service)

	test_name = str(jsonnet_file.relative_to(local_ctx.shared.root_dir))

	# Handle skipped tests
	if jsonnet_file.with_suffix('.skip').exists():
		desc = ya_test_runner.test.Description(
			name=test_name,
			needed_services=frozenset(needed_services),
			tags=frozenset(tags),
		)
		ctx.add_case(
			ya_test_runner.test.StepsCase(
				description=desc,
				steps=[IntegrationSkipStep(test_name)],
			)
		)
		return

	# Parse jsonnet at collection time
	jsonnet_result = _jsonnet.evaluate_file(
		str(jsonnet_file), jpathdir=[str(TEMPLATES_DIR.parent)]
	)
	jsonnet_parsed = json.loads(jsonnet_result)
	extra_tags = jsonnet_parsed.get('tags', [])
	if extra_tags:
		tags.update(extra_tags)

	# Recompute needed_services after tags update
	needed_services = {manager_service}
	if 'stable' not in tags:
		needed_services.add(modules_service)
		needed_services.add(webdriver_service)

	# Compute tmp_dir and unfold config
	tmp_dir = local_ctx.shared.artifacts_dir.joinpath(
		'integration', rel_path
	).with_suffix('')

	jsonnet_conf = _unfold_conf(
		jsonnet_parsed,
		{
			'jsonnetDir': str(jsonnet_file.parent),
			'fileBaseName': jsonnet_file.stem,
			'tmpDir': str(tmp_dir),
		},
	)

	top_level_conf = {
		k: v for k, v in jsonnet_conf.items() if k not in _TOP_LEVEL_METADATA_KEYS
	}
	entries = jsonnet_conf['entry']
	flat_steps = _flatten_tree(entries)

	is_unstable = 'unstable' in tags
	max_attempts = 3 if is_unstable else 1

	frozen_services = frozenset(needed_services)
	frozen_tags = frozenset(tags)

	# Create prepare case
	prepare_name = f'{test_name}/prepare'
	prepare_desc = ya_test_runner.test.Description(
		name=prepare_name,
		needed_services=frozen_services,
		tags=frozen_tags,
	)
	ctx.add_case(
		IntegrationPrepareCase(
			description=prepare_desc,
			jsonnet_path=jsonnet_file,
			top_level_conf=top_level_conf,
			tmp_dir=tmp_dir,
		)
	)

	# Build tree_path -> step name map
	step_names: dict[str, str] = {}
	for tree_path, _dep, single_conf in flat_steps:
		step_names[tree_path] = f'{test_name}/{tree_path}'

	# Create one test case per step
	total_steps = len(flat_steps)
	for tree_path, depends_on_tree_path, single_conf in flat_steps:
		step_name = step_names[tree_path]
		is_benchmark = single_conf.get('benchmark', False)

		if depends_on_tree_path is None:
			deps = frozenset([prepare_name])
		else:
			deps = frozenset([step_names[depends_on_tree_path]])

		step_desc = ya_test_runner.test.Description(
			name=step_name,
			needed_services=frozen_services,
			tags=frozen_tags,
			depends_on=deps,
		)

		# parent_tree_path from step_conf (original jsonnet base path,
		# e.g. "0") is needed for storage path resolution in _run_single_step
		ctx.add_case(
			IntegrationSingleCase(
				description=step_desc,
				jsonnet_path=jsonnet_file,
				manager_service=manager_service,
				tree_path=tree_path,
				parent_tree_path=single_conf.get('parent_tree_path'),
				single_conf=single_conf,
				total_steps=total_steps,
				tmp_dir=tmp_dir,
				max_attempts=max_attempts,
				is_benchmark=is_benchmark,
			)
		)


local_ctx.plugins['integration_test'] = integration_test
