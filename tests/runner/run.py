#!/usr/bin/env python3

# Standard library imports
import argparse
import asyncio
import base64
import concurrent.futures as cfutures
from datetime import datetime
import json
import os
import pickle
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback
from pathlib import Path
from threading import Lock
import typing

# Third-party imports
import _jsonnet
import aiohttp

# Project imports (after path setup)
# These will be imported after path configuration


class Config:
	"""Configuration and constants for the test runner."""

	MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'

	def __init__(self):
		self.script_dir = Path(__file__).parent.absolute()

		# Find monorepo root
		self.root_dir = self.script_dir
		while not self.root_dir.joinpath(self.MONO_REPO_ROOT_FILE).exists():
			self.root_dir = self.root_dir.parent

		self.monorepo_conf = json.loads(
			self.root_dir.joinpath(self.MONO_REPO_ROOT_FILE).read_text()
		)

		# Set up paths
		sys.path.append(str(self.root_dir.joinpath(*self.monorepo_conf['py-std'])))
		sys.path.append(str(self.script_dir))

		self.cases_dir = self.script_dir.parent.joinpath('cases')
		self.root_tmp_dir = self.root_dir.joinpath('build', 'genvm-testdata-out')
		self.root_tmp_dir.mkdir(exist_ok=True, parents=True)

		# Parse arguments
		self.args = self._parse_args()
		self.manager = self.args.manager
		self.file_re = re.compile(self.args.filter)

		# Concurrency settings
		self.max_workers = max(1, (os.cpu_count() or 1) - 2)
		self.run_semaphore = threading.BoundedSemaphore(self.max_workers)

		# State
		self.interrupted = False

		# Logging levels
		self.level_to_num = {
			'trace': 10,
			'debug': 20,
			'info': 30,
			'warning': 40,
			'error': 50,
		}

	def _parse_args(self):
		parser = argparse.ArgumentParser('genvm-test-runner')
		parser.add_argument('--manager', metavar='URI')
		parser.add_argument('--filter', metavar='REGEX', default='.*')
		parser.add_argument('--show-steps', default=False, action='store_true')
		parser.add_argument('--ci', default=False, action='store_true')
		parser.add_argument('--log-level', metavar='LEVEL', default='info')
		parser.add_argument('--no-sequential', default=False, action='store_true')
		parser.add_argument('--start-manager', default=False, action='store_true')
		parser.add_argument('--start-modules', default=False, action='store_true')
		parser.add_argument('--executor-version', default='vTEST')
		res = parser.parse_args()
		if res.manager is None and not res.start_manager:
			parser.error('--manager is required if --start-manager is not given')
		return res


# Initialize configuration
config = Config()

# Import project modules after path setup
from genlayer.py import calldata
from genlayer.py.types import Address
from mock_host import MockHost, MockStorage
import origin.base_host as base_host


class ModuleManager:
	"""Handles module startup operations."""

	def __init__(self, manager_uri: str):
		self.manager_uri = manager_uri

	async def start_module(self, name: str) -> dict:
		async with aiohttp.request(
			'POST',
			f'{self.manager_uri}/module/start',
			json={'module_type': name, 'config': None},
		) as resp:
			body = await resp.json()
			if resp.status != 200 and body != {'error': 'module_already_running'}:
				raise Exception(f'starting module {name} failed: {resp.status} {body}')
			return body

	async def start_default_modules(self):
		await asyncio.gather(self.start_module('Llm'), self.start_module('Web'))


def setup_signal_handler():
	"""Set up signal handler for graceful shutdown."""

	def stop_handler(_, frame):
		config.interrupted = True
		print('interrupted, stopping..', file=sys.stderr)

	signal.signal(signal.SIGINT, stop_handler)


# Set up signal handling
setup_signal_handler()

if config.args.start_manager:
	log_to = config.root_tmp_dir.joinpath('manager.log').open('w')
	manager_process = subprocess.Popen(
		[
			config.root_dir.joinpath('build', 'out', 'bin', 'genvm-modules'),
			'manager',
			'--port',
			'3999',
			'--reroute-to',
			config.args.executor_version,
			'--die-with-parent',
		],
		stdin=subprocess.DEVNULL,
		stdout=log_to,
		stderr=log_to,
	)
	config.args.manager = 'http://localhost:3999'
	config.manager = 'http://localhost:3999'
	time.sleep(2)  # Wait a bit for the manager to start

# Start modules if requested
if config.args.start_modules:
	module_manager = ModuleManager(config.manager)
	asyncio.run(module_manager.start_default_modules())

print(f'concurrency is set to {config.max_workers}')


class LoggerWithLock(base_host.Logger):
	"""Thread-safe logger implementation."""

	def __init__(self, min_level: str, config: Config):
		self.min_level = config.level_to_num[min_level]
		self.prnt_mutex = Lock()
		self.config = config

	def _log_dflt(self, o):
		if isinstance(o, Exception):
			return str(o)
		return o

	def log(self, level: str, msg: str, **kwargs) -> None:
		if self.config.level_to_num[level] < self.min_level:
			return
		with self.prnt_mutex:
			json.dump(
				{
					'message': msg,
					'level': level,
					**kwargs,
				},
				sys.stderr,
				default=self._log_dflt,
			)
			sys.stderr.write('\n')
			sys.stderr.flush()


class ConfigProcessor:
	"""Handles configuration processing and variable substitution."""

	@staticmethod
	def unfold_conf(x: typing.Any, vars: dict[str, str]) -> typing.Any:
		"""Recursively substitute variables in configuration."""
		if isinstance(x, str):
			return re.sub(r'\$\{[a-zA-Z\-_]+\}', lambda m: vars[m.group()[2:-1]], x)
		if isinstance(x, list):
			return [ConfigProcessor.unfold_conf(item, vars) for item in x]
		if isinstance(x, dict):
			return {k: ConfigProcessor.unfold_conf(v, vars) for k, v in x.items()}
		return x


class COLORS:
	"""Terminal color constants."""

	HEADER = '\033[95m'
	OKBLUE = '\033[94m'
	OKCYAN = '\033[96m'
	OKGREEN = '\033[92m'
	WARNING = '\033[93m'
	FAIL = '\033[91m'
	ENDC = '\033[0m'
	BOLD = '\033[1m'
	UNDERLINE = '\033[4m'


class TestReporter:
	"""Handles test result reporting and statistics."""

	def __init__(self, config: Config):
		self.config = config
		self.prnt_mutex = Lock()
		self.categories = {
			'skip': 0,
			'pass': 0,
			'fail': [],
		}
		self.sign_by_category = {
			'skip': '⚠ ',
			'pass': f'{COLORS.OKGREEN}✓{COLORS.ENDC}',
			'fail': f'{COLORS.FAIL}✗{COLORS.ENDC}',
		}

	def report_single(self, path: str, res: dict):
		"""Report a single test result."""
		if res['category'] == 'fail':
			self.categories['fail'].append(str(path))
		else:
			self.categories[res['category']] += 1

		self._print_result(path, res)

	def _print_result(self, path: str, res: dict):
		"""Print formatted test result."""
		with self.prnt_mutex:
			elapsed = res.get('elapsed')
			if elapsed:
				elapsed = f'{elapsed:.3f}s'
			else:
				elapsed = 'NaN'
			print(f"{self.sign_by_category[res['category']]} {path} in {elapsed}")

			if 'reason' in res:
				for line in map(lambda x: '\t' + x, res['reason'].split('\n')):
					print(line)

			if 'exc' in res:
				exc = res['exc']
				if not isinstance(exc, list):
					exc = [exc]
				for e in exc:
					st = traceback.format_exception(e)
					print(re.sub(r'^', '\t\t', ''.join(st), flags=re.MULTILINE))

			if res['category'] == 'fail' and 'steps' in res or self.config.args.show_steps:
				import shlex

				print('\tsteps to reproduce:')
				for line in res['steps']:
					print(f"\t\t{' '.join(map(lambda x: shlex.quote(str(x)), line))}")

			if res['category'] == 'fail':

				def print_lines(st):
					lines = st.splitlines()
					if self.config.args.ci:
						for line in lines:
							print(f'\t\t{line}')
					else:
						for line in lines[:10]:
							print(f'\t\t{line}')
						if len(lines) >= 10:
							print('\t...')

				if 'stdout' in res:
					print('\t=== stdout ===')
					print_lines(res['stdout'])
				if 'stderr' in res:
					print('\t=== stderr ===')
					print_lines(res['stderr'])
				if 'genvm_log' in res:
					print('\t=== genvm_log ===')
					print_lines(res['genvm_log'])

	def print_summary(self):
		"""Print final test summary."""
		print(json.dumps(self.categories))

	def has_failures(self) -> bool:
		"""Check if there were any test failures."""
		return len(self.categories['fail']) != 0


class TestRunner:
	"""Main test runner that orchestrates test execution."""

	def __init__(self, config: Config, reporter: TestReporter):
		self.config = config
		self.reporter = reporter
		self.logger = LoggerWithLock(config.args.log_level, config)

	def run_test(self, jsonnet_rel_path: Path) -> None:
		"""Run a single test from jsonnet configuration."""
		try:
			self._run_test_impl(jsonnet_rel_path)
		except Exception as e:
			e.add_note(f'running {jsonnet_rel_path}')
			raise e

	def _run_test_impl(self, jsonnet_rel_path: Path) -> None:
		"""Implementation of single test execution."""
		debug_path_base = str(jsonnet_rel_path)
		jsonnet_path = self.config.cases_dir.joinpath(jsonnet_rel_path)

		# Check if test should be skipped
		if jsonnet_path.with_suffix('.skip').exists():
			self.reporter.report_single(debug_path_base, {'category': 'skip'})
			return

		# Load and process configuration
		jsonnet_conf = self._load_jsonnet_config(jsonnet_path)
		seq_tmp_dir = self._setup_temp_directory(jsonnet_rel_path)

		# Run preparation if needed
		if 'prepare' in jsonnet_conf[0]:
			self._run_preparation(jsonnet_conf[0]['prepare'])

		# Set up base storage
		empty_storage = self._setup_base_storage(jsonnet_conf[0], seq_tmp_dir)

		# Create run configurations for each step
		run_configs = [
			self._create_run_config(
				i,
				conf_i,
				len(jsonnet_conf),
				jsonnet_rel_path,
				seq_tmp_dir,
				empty_storage,
				debug_path_base,
				jsonnet_path,
			)
			for i, conf_i in enumerate(jsonnet_conf)
		]

		# Execute each configuration step
		for config in run_configs:
			if self.config.interrupted:
				return
			self._execute_test_step(config)

	def _load_jsonnet_config(self, jsonnet_path: Path) -> list:
		"""Load and process jsonnet configuration."""
		jsonnet_conf = _jsonnet.evaluate_file(
			str(jsonnet_path), jpathdir=[str(self.config.script_dir.parent)]
		)
		jsonnet_conf = json.loads(jsonnet_conf)
		if not isinstance(jsonnet_conf, list):
			jsonnet_conf = [jsonnet_conf]

		return ConfigProcessor.unfold_conf(
			jsonnet_conf,
			{'jsonnetDir': str(jsonnet_path.parent), 'fileBaseName': jsonnet_path.stem},
		)

	def _setup_temp_directory(self, jsonnet_rel_path: Path) -> Path:
		"""Set up temporary directory for test execution."""
		seq_tmp_dir = self.config.root_tmp_dir.joinpath(jsonnet_rel_path).with_suffix('')
		shutil.rmtree(seq_tmp_dir, ignore_errors=True)
		seq_tmp_dir.mkdir(exist_ok=True, parents=True)
		return seq_tmp_dir

	def _run_preparation(self, prepare_script: str) -> None:
		"""Run preparation script if specified."""
		subprocess.run(
			[sys.executable, prepare_script],
			stdin=subprocess.DEVNULL,
			stdout=sys.stdout,
			stderr=sys.stderr,
			check=True,
		)

	def _setup_base_storage(self, first_conf: dict, seq_tmp_dir: Path) -> Path:
		"""Set up base mock storage for the test."""
		base_mock_storage = MockStorage()

		if storage_json := first_conf.get('storage_json'):
			storage_b64 = json.loads(Path(storage_json).read_text())
			base_mock_storage._storages = {
				Address(a): {
					base64.b64decode(k): bytearray(base64.b64decode(v)) for k, v in kv.items()
				}
				for a, kv in storage_b64.items()
			}
			empty_storage = seq_tmp_dir.joinpath('empty-storage.pickle')
			with open(empty_storage, 'wb') as f:
				pickle.dump(base_mock_storage, f)

			return empty_storage

		for addr, account_info in first_conf['accounts'].items():
			code = account_info.get('code')
			if code is None:
				continue

			addr = base64.b64decode(addr)
			code_path = self._process_code_file(code, seq_tmp_dir)
			code_bytes = Path(code_path).read_bytes()
			timestamp = first_conf['message'].get('datetime', '2024-11-26T06:42:42.424242Z')
			timestamp = datetime.fromisoformat(timestamp)
			writes = asyncio.run(
				base_host.get_pre_deployment_writes(code_bytes, timestamp, self.config.manager)
			)
			for slot, off, data in writes:
				base_mock_storage.write(Address(addr), slot, off, data)

		empty_storage = seq_tmp_dir.joinpath('empty-storage.pickle')
		with open(empty_storage, 'wb') as f:
			pickle.dump(base_mock_storage, f)

		return empty_storage

	def _process_code_file(self, code: str, tmp_dir: Path) -> str:
		"""Process code file, converting WAT to WASM if needed."""
		if code.endswith('.wat'):
			out_path = tmp_dir.joinpath(Path(code).with_suffix('.wasm').name)
			subprocess.run(
				[
					'wat2wasm',
					'--enable-tail-call',
					'--enable-annotations',
					'-o',
					out_path,
					code,
				],
				check=True,
			)
			return str(out_path)
		return code

	def _create_run_config(
		self,
		i: int,
		single_conf: dict,
		total_conf: int,
		jsonnet_rel_path: Path,
		seq_tmp_dir: Path,
		empty_storage: Path,
		debug_path_base: str,
		jsonnet_path: Path,
	) -> dict:
		"""Create run configuration for a single test step."""
		single_conf = pickle.loads(pickle.dumps(single_conf))  # Deep copy

		if total_conf == 1:
			my_tmp_dir = seq_tmp_dir
			suff = ''
			my_debug_path = debug_path_base
		else:
			my_tmp_dir = seq_tmp_dir.joinpath(str(i))
			suff = f'.{i}'
			my_debug_path = debug_path_base + f' ({i})'

		my_tmp_dir.mkdir(exist_ok=True, parents=True)

		# Set up storage paths
		if i == 0:
			pre_storage = empty_storage
		else:
			pre_storage = seq_tmp_dir.joinpath(str(i - 1), 'storage.pickle')
		post_storage = my_tmp_dir.joinpath('storage.pickle')

		# Process account code files
		for acc_val in single_conf['accounts'].values():
			code_path = acc_val.get('code', None)
			if code_path is not None:
				acc_val['code'] = self._process_code_file(code_path, my_tmp_dir)

		# Prepare calldata
		calldata_bytes = calldata.encode(
			eval(
				single_conf['calldata'],
				globals(),
				single_conf['vars'].copy(),
			)
		)

		# Set up paths
		messages_path = my_tmp_dir.joinpath('messages.txt')
		mock_sock_path = Path(
			'/tmp', 'genvm-test', jsonnet_rel_path.with_suffix(f'.sock{suff}')
		)
		mock_sock_path.parent.mkdir(exist_ok=True, parents=True)

		# Create mock host
		host = MockHost(
			path=str(mock_sock_path),
			calldata=calldata_bytes,
			storage_path_post=post_storage,
			storage_path_pre=pre_storage,
			leader_nondet=single_conf.get('leader_nondet', None),
			messages_path=messages_path,
			balances={Address(k): v for k, v in single_conf.get('balances', {}).items()},
			running_address=Address(single_conf['message']['contract_address']),
		)

		mock_host_path = my_tmp_dir.joinpath('mock-host.pickle')
		mock_host_path.write_bytes(pickle.dumps(host))

		return {
			'host': host,
			'message': single_conf['message'],
			'sync': single_conf.get('sync', False),
			'tmp_dir': my_tmp_dir,
			'expected_output': jsonnet_path.with_suffix(f'{suff}.stdout'),
			'suff': suff,
			'mock_host_path': mock_host_path,
			'messages_path': messages_path,
			'expected_messages_path': jsonnet_path.with_suffix(f'{suff}.msgs'),
			'deadline': single_conf.get('deadline', 10 * 60),  # 10 minutes
			'test_name': my_debug_path,
		}

	def _execute_test_step(self, config: dict) -> None:
		"""Execute a single test step."""
		test_name = config['test_name']
		tmp_dir = config['tmp_dir']
		steps = []

		logger = self.logger.with_keys({'test': test_name})

		with config['host'] as mock_host:
			time_start = time.monotonic()
			try:
				res = asyncio.run(
					base_host.run_genvm(
						mock_host,
						manager_uri=self.config.manager,
						message=config['message'],
						timeout=config['deadline'],
						capture_output=True,
						is_sync=config['sync'],
						host_data='{"node_address": "0x", "tx_id": "0x"}',
						logger=logger,
						host='unix://' + config['host'].path,
						extra_args=['--debug-mode', '--print=result'],
					)
				)
			except Exception as e:
				time_elapsed = time.monotonic() - time_start
				self.reporter.report_single(
					test_name,
					{
						'category': 'fail',
						'steps': steps,
						'exception': 'internal error',
						'exc': e,
						'elapsed': time_elapsed,
						**e.args[-1],
					},
				)
				return

		time_elapsed = time.monotonic() - time_start
		base_result = {
			'steps': steps,
			'stdout': res.stdout,
			'stderr': res.stderr,
			'genvm_log': res.genvm_log,
			'elapsed': time_elapsed,
		}

		# Save outputs to files
		self._save_test_outputs(tmp_dir, res)

		# Validate outputs
		if not self._validate_stdout(config, res.stdout, base_result, test_name):
			return
		if not self._validate_messages(config, base_result, test_name):
			return

		self.reporter.report_single(test_name, {'category': 'pass', **base_result})

	def _save_test_outputs(self, tmp_dir: Path, res) -> None:
		"""Save test outputs to files."""
		got_stdout_path = tmp_dir.joinpath('stdout.txt')
		got_stdout_path.parent.mkdir(parents=True, exist_ok=True)
		got_stdout_path.write_text(res.stdout)
		tmp_dir.joinpath('stderr.txt').write_text(res.stderr)
		tmp_dir.joinpath('genvm.log').write_text(
			'\n'.join(json.dumps(x) for x in res.genvm_log)
		)

	def _validate_stdout(
		self, config: dict, actual_stdout: str, base_result: dict, test_name: str
	) -> bool:
		"""Validate stdout output against expected."""
		exp_stdout_path = config['expected_output']
		if exp_stdout_path.exists():
			if exp_stdout_path.read_text() != actual_stdout:
				got_stdout_path = config['tmp_dir'].joinpath('stdout.txt')
				self.reporter.report_single(
					test_name,
					{
						'category': 'fail',
						'reason': f'stdout mismatch, see\n\tdiff {str(exp_stdout_path)} {str(got_stdout_path)}',
						**base_result,
					},
				)
				return False
		else:
			exp_stdout_path.write_text(actual_stdout)
		return True

	def _validate_messages(self, config: dict, base_result: dict, test_name: str) -> bool:
		"""Validate messages output against expected."""
		messages_path = config['messages_path']
		expected_messages_path = config['expected_messages_path']

		if messages_path.exists() != expected_messages_path.exists():
			self.reporter.report_single(
				test_name,
				{
					'category': 'fail',
					'reason': f'messages do not exists\n\tdiff {messages_path} {expected_messages_path}',
					**base_result,
				},
			)
			return False

		if messages_path.exists():
			got = messages_path.read_text()
			exp = expected_messages_path.read_text()
			if got != exp:
				self.reporter.report_single(
					test_name,
					{
						'category': 'fail',
						'reason': f'messages differ\n\tdiff {messages_path} {expected_messages_path}',
						**base_result,
					},
				)
				return False
		return True


class TestExecutor:
	"""Orchestrates the execution of multiple tests with concurrency control."""

	def __init__(self, config: Config, test_runner: TestRunner, reporter: TestReporter):
		self.config = config
		self.test_runner = test_runner
		self.reporter = reporter

	def run_with_semaphore(self, *args, **kwargs):
		"""Run test with semaphore for concurrency control."""
		# Semaphore is needed even with thread pool executor
		# due to strange behavior on CI
		with self.config.run_semaphore:
			return self.test_runner.run_test(*args, **kwargs)

	def get_test_files(self) -> list[Path]:
		"""Get list of test files matching the filter."""
		files = [
			x.relative_to(self.config.cases_dir)
			for x in self.config.cases_dir.glob('**/*.jsonnet')
		]
		files = [x for x in files if self.config.file_re.search(str(x)) is not None]
		files.sort()
		return files

	def process_result(self, path: Path, res_getter):
		"""Process test result and handle exceptions."""
		try:
			res_getter()
		except Exception as e:
			res = {
				'category': 'fail',
				'reason': str(e),
				'exc': e,
			}
			self.reporter.report_single(str(path), res)

	def run_all_tests(self):
		"""Run all tests with appropriate concurrency."""
		files = self.get_test_files()

		if not files:
			self.reporter.print_summary()
			return

		# Split files into sequential and parallel groups
		# NOTE: Sequential execution is needed to cache wasm compilation result
		if self.config.args.no_sequential:
			firsts = []
			lasts = files
		else:
			firsts = [f for f in files if f.name.startswith('_hello')]
			lasts = [f for f in files if not f.name.startswith('_hello')]
			if len(firsts) == 0:
				firsts = [files[0]]
				lasts = files[1:]

		print(
			f'running the first test(s) sequentially ({len(firsts)}), it can take a while..'
		)

		# Run first tests sequentially
		for f in firsts:
			try:
				self.process_result(f, lambda: self.test_runner.run_test(f))
			except Exception as e:
				e.add_note(f'in file {f}')
				raise e

		# Run remaining tests in parallel
		with cfutures.ThreadPoolExecutor(self.config.max_workers) as executor:
			future2path = {
				executor.submit(self.run_with_semaphore, path): path for path in lasts
			}
			for future in cfutures.as_completed(future2path):
				path = future2path[future]
				try:
					self.process_result(future2path[future], lambda: future.result())
				except Exception as e:
					e.add_note(f'in file {path}')
					raise e

		# Generate reports
		self.reporter.print_summary()


def main():
	"""Main entry point for the test runner."""
	reporter = TestReporter(config)
	test_runner = TestRunner(config, reporter)
	executor = TestExecutor(config, test_runner, reporter)

	try:
		executor.run_all_tests()
	finally:
		if reporter.has_failures():
			sys.exit(1)
		sys.exit(0)


if __name__ == '__main__':
	main()
