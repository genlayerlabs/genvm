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
from ya_test_runner.test import CONST_PASSED

# Get the local context
local_ctx = ya_test_runner.stage.configuration.current_context()

# Load build info
build_info = json.loads(
	local_ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
)

BUILD_DIR = Path(build_info['build_dir'])
TARGET_DIR = Path(build_info['rust_target_dir'])

# Set up paths for importing runner modules
TESTS_DIR = local_ctx.shared.root_dir.joinpath('tests')
RUNNER_DIR = TESTS_DIR.joinpath('runner')
CASES_DIR = TESTS_DIR.joinpath('cases')
TEMPLATES_DIR = TESTS_DIR.joinpath('templates')

# Lazy imports - these are only loaded when actually running tests
_lazy_imports_loaded = False
gvm_calldata = None
Address = None
MockHost = None
MockStorage = None
base_host = None


def _load_lazy_imports():
	"""Load test-time dependencies lazily to avoid import errors at configuration time."""
	global _lazy_imports_loaded, gvm_calldata, Address, MockHost, MockStorage, base_host

	if _lazy_imports_loaded:
		return

	# Add runner to path so we can import MockHost etc.
	if str(RUNNER_DIR) not in sys.path:
		sys.path.insert(0, str(RUNNER_DIR))

	# Find py-std location from monorepo config
	MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
	monorepo_conf = json.loads(
		local_ctx.shared.root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text()
	)
	py_std_path = local_ctx.shared.root_dir.joinpath(*monorepo_conf['py-std'])
	if str(py_std_path) not in sys.path:
		sys.path.insert(0, str(py_std_path))

	# Now we can import the required modules
	from genlayer.py import calldata as _gvm_calldata
	from genlayer.py.types import Address as _Address
	from mock_host import MockHost as _MockHost, MockStorage as _MockStorage
	import origin.base_host as _base_host

	gvm_calldata = _gvm_calldata
	Address = _Address
	MockHost = _MockHost
	MockStorage = _MockStorage
	base_host = _base_host

	_lazy_imports_loaded = True


# Default environment for tests
default_env = {
	k: v
	for k, v in os.environ.items()
	if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
}


def _unfold_conf(x: typing.Any, vars: dict[str, str]) -> typing.Any:
	"""Recursively substitute variables in configuration."""
	if isinstance(x, str):
		return re.sub(r'\$\{[a-zA-Z\-_]+\}', lambda m: vars[m.group()[2:-1]], x)
	if isinstance(x, list):
		return [_unfold_conf(item, vars) for item in x]
	if isinstance(x, dict):
		return {k: _unfold_conf(v, vars) for k, v in x.items()}
	return x


@dataclass
class IntegrationTestCase(ya_test_runner.test.Case):
	"""Integration test case wrapping a jsonnet test definition."""

	description: ya_test_runner.test.Description
	jsonnet_path: Path
	manager_service: ya_test_runner.stage.collection.Service
	webdriver_service: ya_test_runner.stage.collection.Service

	async def into_steps(self) -> list[ya_test_runner.exec.step.Step]:
		steps = []
		steps.append(IntegrationTestStep(self))
		return steps


class IntegrationTestStep(ya_test_runner.exec.step.Python):
	"""Executes all steps of an integration test."""

	def __init__(self, test_case: IntegrationTestCase):
		self._test_case = test_case

	def to_str(self) -> str:
		return f'<integration test: {self._test_case.jsonnet_path.name}>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		import time

		start_time = time.monotonic()

		try:
			result = await self._run_test()
			elapsed = time.monotonic() - start_time
			return ya_test_runner.test.Result(
				passed=result['passed'],
				context=result.get('context', {}),
				elapsed_seconds=elapsed,
			)
		except Exception as e:
			elapsed = time.monotonic() - start_time
			return ya_test_runner.test.Result(
				passed=False,
				context={'exception': str(e), 'exception_type': type(e).__name__},
				elapsed_seconds=elapsed,
			)

	async def _run_test(self) -> dict:
		import _jsonnet

		# Load lazy imports
		_load_lazy_imports()

		jsonnet_path = self._test_case.jsonnet_path

		# Check for skip file
		if jsonnet_path.with_suffix('.skip').exists():
			return {'passed': True, 'context': {'skipped': True}}

		# Load jsonnet configuration
		jsonnet_conf = _jsonnet.evaluate_file(
			str(jsonnet_path), jpathdir=[str(TEMPLATES_DIR.parent)]
		)
		jsonnet_conf = json.loads(jsonnet_conf)
		if not isinstance(jsonnet_conf, list):
			jsonnet_conf = [jsonnet_conf]

		jsonnet_conf = _unfold_conf(
			jsonnet_conf,
			{'jsonnetDir': str(jsonnet_path.parent), 'fileBaseName': jsonnet_path.stem},
		)

		# Set up temp directory
		rel_path = jsonnet_path.relative_to(CASES_DIR)
		tmp_dir = BUILD_DIR.joinpath('genvm-testdata-out', rel_path).with_suffix('')
		shutil.rmtree(tmp_dir, ignore_errors=True)
		tmp_dir.mkdir(exist_ok=True, parents=True)

		# Run preparation if needed
		if 'prepare' in jsonnet_conf[0]:
			import subprocess

			subprocess.run(
				[sys.executable, jsonnet_conf[0]['prepare']],
				stdin=subprocess.DEVNULL,
				check=True,
			)

		# Set up base storage
		base_mock_storage = MockStorage()
		if storage_json := jsonnet_conf[0].get('storage_json'):
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

		# Run each step
		for i, single_conf in enumerate(jsonnet_conf):
			result = await self._run_single_step(
				i,
				single_conf,
				len(jsonnet_conf),
				jsonnet_path,
				tmp_dir,
				empty_storage,
			)
			if not result['passed']:
				return result

		return {'passed': True, 'context': {}}

	async def _run_single_step(
		self,
		step_index: int,
		single_conf: dict,
		total_steps: int,
		jsonnet_path: Path,
		seq_tmp_dir: Path,
		empty_storage: Path,
	) -> dict:
		import subprocess

		single_conf = pickle.loads(pickle.dumps(single_conf))  # Deep copy

		if total_steps == 1:
			my_tmp_dir = seq_tmp_dir
			suff = ''
		else:
			my_tmp_dir = seq_tmp_dir.joinpath(str(step_index))
			suff = f'.{step_index}'

		my_tmp_dir.mkdir(exist_ok=True, parents=True)

		# Set up storage paths
		if step_index == 0:
			pre_storage = empty_storage
		else:
			pre_storage = seq_tmp_dir.joinpath(str(step_index - 1), 'storage.pickle')
		post_storage = my_tmp_dir.joinpath('storage.pickle')

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
				subprocess.run(
					[
						'wat2wasm',
						'--enable-tail-call',
						'--enable-annotations',
						'-o',
						out_path,
						code_path,
					],
					check=True,
				)
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

		# Set up paths
		messages_path = my_tmp_dir.joinpath('messages.txt')
		rel_path = jsonnet_path.relative_to(CASES_DIR)
		mock_sock_path = Path('/tmp', 'genvm-test', rel_path.with_suffix(f'.sock{suff}'))
		mock_sock_path.parent.mkdir(exist_ok=True, parents=True)

		# Create mock host
		host = MockHost(
			path=str(mock_sock_path),
			storage_path_post=post_storage,
			storage_path_pre=pre_storage,
			leader_nondet=single_conf.get('leader_nondet', None),
			messages_path=messages_path,
			balances={Address(k): v for k, v in single_conf.get('balances', {}).items()},
			running_address=single_conf['message']['contract_address'],
		)

		# Get manager URI from the service
		manager_svc = self._test_case.manager_service.manager
		manager_uri = f'http://localhost:{manager_svc.port}'

		# Run the test
		with host as mock_host:
			try:
				res = await base_host.run_genvm(
					mock_host,
					manager_uri=manager_uri,
					message=single_conf['message'],
					timeout=single_conf.get('deadline', 10 * 60),
					capture_output=True,
					is_sync=single_conf.get('sync', False),
					host_data='{"node_address": "0x", "tx_id": "0x"}',
					logger=base_host.NoLogger(),
					host='unix://' + mock_host.path,
					extra_args=['--debug-mode', '--print=result'],
					code=code,
					calldata=calldata_bytes,
				)
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
						'exception': str(e),
						'step': step_index,
					},
				}

		# Save outputs
		my_tmp_dir.joinpath('stdout.txt').write_text(res.stdout)
		my_tmp_dir.joinpath('stderr.txt').write_text(res.stderr)
		my_tmp_dir.joinpath('genvm.log').write_text(
			'\n'.join(json.dumps(x) for x in res.genvm_log)
		)

		# Validate stdout
		exp_stdout_path = jsonnet_path.with_suffix(f'{suff}.stdout')
		if exp_stdout_path.exists():
			if exp_stdout_path.read_text() != res.stdout:
				return {
					'passed': False,
					'context': {
						'reason': 'stdout mismatch',
						'expected_path': str(exp_stdout_path),
						'got_path': str(my_tmp_dir.joinpath('stdout.txt')),
						'stdout': res.stdout,
						'stderr': res.stderr,
					},
				}
		else:
			# Create expected output file
			exp_stdout_path.write_text(res.stdout)

		# Validate messages
		expected_messages_path = jsonnet_path.with_suffix(f'{suff}.msgs')
		if messages_path.exists() != expected_messages_path.exists():
			return {
				'passed': False,
				'context': {
					'reason': 'messages existence mismatch',
					'messages_path': str(messages_path),
					'expected_messages_path': str(expected_messages_path),
				},
			}

		if messages_path.exists():
			got = messages_path.read_text()
			exp = expected_messages_path.read_text()
			if got != exp:
				return {
					'passed': False,
					'context': {
						'reason': 'messages differ',
						'messages_path': str(messages_path),
						'expected_messages_path': str(expected_messages_path),
					},
				}

		return {'passed': True, 'context': {}}


def _test_needs_webdriver(jsonnet_path: Path) -> bool:
	"""Check if a test needs webdriver based on its content."""
	content = jsonnet_path.read_text()
	# Web tests typically involve screenshots or webpage interactions
	return 'screenshot' in content.lower() or 'get_webpage' in content.lower()


def integration_test(
	ctx: ya_test_runner.stage.collection.Context,
	*,
	manager_service: ya_test_runner.stage.collection.Service,
	modules_service: ya_test_runner.stage.collection.Service | None = None,
	webdriver_service: ya_test_runner.stage.collection.Service | None = None,
) -> None:
	"""
	Collect integration tests from tests/cases/ directory.

	Args:
		ctx: Collection context
		manager_service: The manager service to use for all tests
		modules_service: The modules service (Llm, Web) for unstable/semi-stable tests
		webdriver_service: Optional webdriver service for web tests
	"""
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

		# Determine needed services based on stability
		# - stable tests: just manager
		# - unstable/semi-stable tests: manager + modules + (optionally webdriver)
		needed_services: set[ya_test_runner.stage.collection.Service] = {manager_service}

		needs_modules = stability_tag in ('unstable', 'semi-stable')
		if needs_modules and modules_service is not None:
			needed_services.add(modules_service)

		if webdriver_service is not None and _test_needs_webdriver(jsonnet_file):
			needed_services.add(webdriver_service)
			tags.add('web')
			# Web tests also need modules
			if modules_service is not None:
				needed_services.add(modules_service)

		desc = ya_test_runner.test.Description(
			name=str(jsonnet_file.relative_to(local_ctx.shared.root_dir)),
			needed_services=frozenset(needed_services),
			tags=frozenset(tags),
			console_pool=False,  # Integration tests can run in parallel
		)

		case = IntegrationTestCase(
			description=desc,
			jsonnet_path=jsonnet_file,
			manager_service=manager_service,
			webdriver_service=webdriver_service,
		)

		ctx.add_case(case)


local_ctx.plugins['integration_test'] = integration_test
