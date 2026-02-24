"""
Integration test plugin for ya-test-runner.

This plugin collects and runs integration tests from .jsonnet files in tests/cases/.
It uses the same MockHost/base_host infrastructure as the old runner.
"""

import base64
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

# Set up paths for importing plugin modules
TESTS_DIR = local_ctx.shared.root_dir.joinpath('tests')
CASES_DIR = TESTS_DIR.joinpath('cases')
TEMPLATES_DIR = TESTS_DIR.joinpath('templates')

import genlayer.py.calldata as gvm_calldata
from genlayer.py.types import Address
from gvm_extra.mock_host import MockHost as MockHost, MockStorage as MockStorage
import origin.base_host as base_host
import origin.logger as origin_logger
import origin.public_abi as public_abi

# Default environment for tests
default_env = {
	k: v
	for k, v in os.environ.items()
	if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
}


def _make_log_adapter(formatter: ya_test_runner.Formatter) -> 'origin_logger.Logger':
	class _FormatterLoggerAdapter(origin_logger.Logger):
		"""Adapts ya_test_runner.formatter.Formatter to base_host.Logger interface."""

		def __init__(self, formatter: ya_test_runner.Formatter):
			self._formatter = formatter

		def log(self, level: str, msg: str, **kwargs) -> None:
			fmt_level = ya_test_runner.Formatter.Level.from_str(level)
			self._formatter.log(fmt_level, msg, **kwargs)

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
	"""DFS-traverse the step tree and return (tree_path, parent_tree_path, step_conf) tuples."""
	result = []
	for entry in entries:
		step_conf = {k: v for k, v in entry.items() if k != 'next'}
		tree_path = step_conf['tree_path']
		parent_tree_path = step_conf.get('parent_tree_path')
		result.append((tree_path, parent_tree_path, step_conf))
		if 'next' in entry:
			result.extend(_flatten_tree(entry['next']))
	return result


@dataclass
class IntegrationSetupResult:
	"""Result from IntegrationSetupStep, passed to subsequent steps."""

	tmp_dir: Path
	empty_storage: Path
	skipped: bool = False


_TOP_LEVEL_METADATA_KEYS = frozenset({'entry', 'tags'})


@dataclass
class IntegrationTestCase(ya_test_runner.test.Case):
	"""Integration test case wrapping a jsonnet test definition."""

	description: ya_test_runner.test.Description
	jsonnet_path: Path
	manager_service: ya_test_runner.stage.collection.Service
	webdriver_service: ya_test_runner.stage.collection.Service
	jsonnet_parsed: dict | None = None

	async def into_steps(self) -> list[ya_test_runner.exec.step.Step]:
		jsonnet_path = self.jsonnet_path

		# Check for skip file early
		if jsonnet_path.with_suffix('.skip').exists():
			return [IntegrationSkipStep(self)]

		# Use cached parse from collection time, or evaluate now
		if self.jsonnet_parsed is not None:
			jsonnet_conf = self.jsonnet_parsed
		else:
			import _jsonnet

			jsonnet_conf = _jsonnet.evaluate_file(
				str(jsonnet_path), jpathdir=[str(TEMPLATES_DIR.parent)]
			)
			jsonnet_conf = json.loads(jsonnet_conf)

		local_ctx.shared.logger.debug(
			'Parsed jsonnet configuration',
			test_name=str(self.description.name),
			jsonnet_path=str(jsonnet_path),
			conf=jsonnet_conf,
		)

		# Calculate paths early so ${tmpDir} is available for variable substitution
		rel_path = jsonnet_path.relative_to(CASES_DIR)
		tmp_dir = local_ctx.shared.artifacts_dir.joinpath(
			'integration', rel_path
		).with_suffix('')

		jsonnet_conf = _unfold_conf(
			jsonnet_conf,
			{
				'jsonnetDir': str(jsonnet_path.parent),
				'fileBaseName': jsonnet_path.stem,
				'tmpDir': str(tmp_dir),
			},
		)

		# Extract top-level fields (exclude metadata-only keys)
		top_level_conf = {
			k: v for k, v in jsonnet_conf.items() if k not in _TOP_LEVEL_METADATA_KEYS
		}
		entries = jsonnet_conf['entry']

		# Flatten tree
		flat_steps = _flatten_tree(entries)

		is_unstable = 'unstable' in self.description.tags
		max_attempts = 3 if is_unstable else 1

		steps: list[ya_test_runner.exec.step.Step] = []

		# Setup step
		steps.append(
			IntegrationSetupStep(
				test_case=self,
				top_level_conf=top_level_conf,
				tmp_dir=tmp_dir,
			)
		)

		# One step per flattened tree node
		total_steps = len(flat_steps)
		for tree_path, parent_tree_path, single_conf in flat_steps:
			is_benchmark = single_conf.get('benchmark', False)
			cur_step = IntegrationSingleStep(
				test_case=self,
				tree_path=tree_path,
				parent_tree_path=parent_tree_path,
				single_conf=single_conf,
				total_steps=total_steps,
				tmp_dir=tmp_dir,
				max_attempts=max_attempts,
			)
			if is_benchmark:
				for i in range(10):
					steps.append(CheckInterruptedStep())
					steps.append(ya_test_runner.test.BenchMeasureStep())
					steps.append(cur_step)
				steps.append(ya_test_runner.test.BenchCollectStep(local_ctx.shared.printer))
			else:
				steps.append(cur_step)

		return steps


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

	def __init__(self, test_case: IntegrationTestCase):
		self._test_case = test_case

	def to_str(self) -> str:
		return f'<skip: {self._test_case.jsonnet_path.name}>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		local_ctx.shared.logger.warning(
			'Test skipped',
			test_name=self._test_case.description.name,
		)
		return ya_test_runner.test.Result(
			passed=True,
			context={'skipped': True},
			elapsed_seconds=0,
		)


class IntegrationSetupStep(ya_test_runner.exec.step.Python):
	"""Sets up the test environment: temp dir, prepare script, base storage."""

	def __init__(
		self,
		test_case: IntegrationTestCase,
		top_level_conf: dict,
		tmp_dir: Path,
	):
		self._test_case = test_case
		self._top_level_conf = top_level_conf
		self._tmp_dir = tmp_dir

	def to_str(self) -> str:
		return f'<setup: {self._test_case.jsonnet_path.name}>'

	async def run(self, previous_results: list[typing.Any]) -> IntegrationSetupResult:
		# Set up temp directory
		shutil.rmtree(self._tmp_dir, ignore_errors=True)
		self._tmp_dir.mkdir(exist_ok=True, parents=True)

		# Run preparation if needed
		if 'prepare' in self._top_level_conf:
			cmd = Command(
				args=[sys.executable, self._top_level_conf['prepare']],
				cwd=self._test_case.jsonnet_path.parent,
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
		if storage_json := self._top_level_conf.get('storage_json'):
			storage_b64 = json.loads(Path(storage_json).read_text())
			base_mock_storage._storages = {
				Address(a): {
					base64.b64decode(k): bytearray(base64.b64decode(v)) for k, v in kv.items()
				}
				for a, kv in storage_b64.items()
			}

		empty_storage = self._tmp_dir.joinpath('empty-storage.pickle')
		with open(empty_storage, 'wb') as f:
			pickle.dump(base_mock_storage, f)

		return IntegrationSetupResult(
			tmp_dir=self._tmp_dir,
			empty_storage=empty_storage,
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


class IntegrationSingleStep(ya_test_runner.exec.step.Python):
	"""Executes a single step of an integration test."""

	def __init__(
		self,
		test_case: IntegrationTestCase,
		tree_path: str,
		parent_tree_path: str | None,
		single_conf: dict,
		total_steps: int,
		tmp_dir: Path,
		max_attempts: int,
	):
		self._test_case = test_case
		self._tree_path = tree_path
		self._parent_tree_path = parent_tree_path
		self._single_conf = single_conf
		self._total_steps = total_steps
		self._tmp_dir = tmp_dir
		self._max_attempts = max_attempts

	def to_str(self) -> str:
		return f'<step {self._tree_path}: {self._test_case.jsonnet_path.name}>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		# Get setup result from first step
		setup_result: IntegrationSetupResult = previous_results[0]

		for attempt in range(self._max_attempts):
			result = await self._run_single_step(setup_result.empty_storage)
			if result['passed']:
				local_ctx.shared.logger.log(
					ya_test_runner.Formatter.Level.INFO
					if self._total_steps > 1
					else ya_test_runner.Formatter.Level.DEBUG,
					f'Test step passed',
					test_name=str(self._test_case.description.name),
					tree_path=self._tree_path,
					total_steps=self._total_steps,
				)
				return ya_test_runner.test.Result(
					passed=True,
					context=result.get('context', {}),
					elapsed_seconds=0,
				)

			if local_ctx.shared.is_interrupted or attempt + 1 >= self._max_attempts:
				# Raise FinishedEarlyException to stop subsequent steps
				raise ya_test_runner.test.FinishedEarlyException(
					result=ya_test_runner.test.Result(
						passed=False,
						context=result.get('context', {}),
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

	async def _run_single_step(self, empty_storage: Path) -> dict:
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
		calldata_bytes = gvm_calldata.encode(
			eval(
				single_conf['calldata'],
				globals(),
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
		messages_path = my_tmp_dir.joinpath('messages.txt')
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

		# Create mock host
		host = MockHost(
			path=str(mock_sock_path),
			storage_path_post=post_storage,
			storage_path_pre=pre_storage,
			leader_nondet=leader_nondet,
			messages_path=messages_path,
			balances={Address(k): v for k, v in single_conf.get('balances', {}).items()},
			running_address=single_conf['message']['contract_address'],
		)

		# Get manager URI from the service
		manager_svc = self._test_case.manager_service
		port = manager_svc.meta['port']
		manager_uri = f'http://localhost:{port}'

		# Run the test
		with host as mock_host:
			try:
				logger = _make_log_adapter(local_ctx.shared.logger)
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
				mode = single_conf.get('mode', 'l')
				res = await base_host.run_genvm(
					mock_host,
					manager_uri=manager_uri,
					message=single_conf['message'],
					timeout=single_conf.get('deadline', 10 * 60),
					capture_output=True,
					is_sync=(mode == 's'),
					host_data=host_data,
					logger=logger,
					host='unix://' + mock_host.path,
					extra_args=['--debug-mode'],
					code=code,
					calldata=calldata_bytes,
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
					return_part += f'executed with `UserError("{res.result_data}")`\n'
				nondet_part = ''
				if mock_host.nondet_disagreement_call_no is not None:
					nondet_part = (
						f'nondet disagreement: {mock_host.nondet_disagreement_call_no}\n'
					)
				res.stdout = stdout_raw + return_part + nondet_part
				# Apply events and storage changes
				for evs in res.result_events:
					mock_host.post_event(evs[:-1], evs[-1])
				for k, v in res.result_storage_changes:
					mock_host.storage.write(
						mock_host.running_address,
						k[:32],
						int.from_bytes(k[32:], byteorder='little'),
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
			if len(host.nondet_results) == 0:
				nondet_list = []
			else:
				size = max(host.nondet_results.keys()) + 1
				nondet_list = [host.nondet_results[i] for i in range(size)]
			with open(path_to_which_leader_puts_result, 'wb') as f:
				pickle.dump(nondet_list, f)

		# Save outputs
		my_tmp_dir.joinpath('stdout.txt').write_text(res.stdout)
		my_tmp_dir.joinpath('stderr.txt').write_text(res.stderr)
		my_tmp_dir.joinpath('genvm.log').write_text(
			'\n'.join(json.dumps(x) for x in res.genvm_log)
		)

		# Save RunHostAndProgramRes pickle
		result_path = Path(single_conf['result_path'])
		result_path.parent.mkdir(exist_ok=True, parents=True)
		with open(result_path, 'wb') as f:
			pickle.dump(res, f)

		# Write hash file (calldata-encoded deterministic result fields)
		hash_data = gvm_calldata.encode(
			(
				int(res.result_kind),
				res.result_data,
				res.result_fingerprint,
				res.result_storage_changes,
				res.result_events,
			)
		)

		# Build semantics from components
		messages_content = messages_path.read_text() if messages_path.exists() else ''
		semantics_parts = {
			'stdout': stdout_raw,
			'return': return_part,
			'nondet': nondet_part,
			'messages': messages_content,
		}
		semantics_components = single_conf['expected_semantics_components']
		if semantics_components:
			semantics = ''.join(semantics_parts[c] for c in semantics_components)

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
							'got_path': str(my_tmp_dir.joinpath('stdout.txt')),
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

		if expected_hash_path is not None:
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

	Args:
		ctx: Collection context
		manager_service: The manager service to use for all tests
		modules_service: The modules service (Llm, Web) for unstable/semi-stable tests
		webdriver_service: Optional webdriver service for web tests
	"""
	import _jsonnet

	jsonnet_files = list(CASES_DIR.glob('**/*.jsonnet'))
	jsonnet_files.sort()

	for jsonnet_file in jsonnet_files:
		rel_path = jsonnet_file.relative_to(CASES_DIR)

		# Determine stability tag from path
		# stable/, unstable/, semi-stable/
		tags: set[str] = {'integration'}
		stability_tag = rel_path.parts[0] if rel_path.parts else 'unknown'
		if stability_tag in ('stable', 'unstable', 'semi-stable'):
			tags.add(stability_tag)
		elif stability_tag.startswith('_'):
			# Files like _hello_world_.jsonnet are treated as stable
			tags.add('stable')
			stability_tag = 'stable'

		# Parse jsonnet at collection time to extract metadata tags and cache result
		jsonnet_parsed = None
		if not jsonnet_file.with_suffix('.skip').exists():
			jsonnet_result = _jsonnet.evaluate_file(
				str(jsonnet_file), jpathdir=[str(TEMPLATES_DIR.parent)]
			)
			jsonnet_parsed = json.loads(jsonnet_result)
			extra_tags = jsonnet_parsed.get('tags', [])
			if extra_tags:
				tags.update(extra_tags)

		needed_services: set[ya_test_runner.stage.collection.Service] = {manager_service}

		if 'stable' not in tags:
			needed_services.add(modules_service)
			needed_services.add(webdriver_service)

		test_name = str(jsonnet_file.relative_to(local_ctx.shared.root_dir))

		desc = ya_test_runner.test.Description(
			name=test_name,
			needed_services=frozenset(needed_services),
			tags=frozenset(tags),
			console_pool=False,  # Integration tests can run in parallel
		)

		case = IntegrationTestCase(
			description=desc,
			jsonnet_path=jsonnet_file,
			manager_service=manager_service,
			webdriver_service=webdriver_service,
			jsonnet_parsed=jsonnet_parsed,
		)

		ctx.add_case(case)


local_ctx.plugins['integration_test'] = integration_test
