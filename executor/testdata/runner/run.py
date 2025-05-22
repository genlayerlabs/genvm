#!/usr/bin/env python3
from pathlib import Path
import concurrent.futures as cfutures
import os
import subprocess
import _jsonnet
import json
from threading import Lock
import argparse
import re
import sys
import pickle
import asyncio

import http.server as httpserv

MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
script_dir = Path(__file__).parent.absolute()
http_dir = str(script_dir.parent.joinpath('http').absolute())

root_dir = script_dir
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent
MONOREPO_CONF = json.loads(root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text())

sys.path.append(str(root_dir.joinpath(*MONOREPO_CONF['py-std'])))
sys.path.append(str(script_dir))

from genlayer.py import calldata
from genlayer.py.types import Address
from mock_host import MockHost, MockStorage, run_host_and_program
import base_host


class MyHTTPHandler(httpserv.SimpleHTTPRequestHandler):
	def __init__(self, *args, **kwargs):
		httpserv.SimpleHTTPRequestHandler.__init__(
			self, *args, **kwargs, directory=http_dir
		)


dir = script_dir.parent.joinpath('cases')
root_tmp_dir = root_dir.joinpath('build', 'genvm-testdata-out')

coverage_dir = root_tmp_dir.joinpath('coverage')
import shutil

shutil.rmtree(coverage_dir, True)
coverage_dir.mkdir(parents=True, exist_ok=True)

arg_parser = argparse.ArgumentParser('genvm-test-runner')
arg_parser.add_argument(
	'--gen-vm',
	metavar='EXE',
	default=str(
		Path(os.getenv('GENVM', root_dir.joinpath('build', 'out', 'bin', 'genvm')))
	),
)
arg_parser.add_argument('--filter', metavar='REGEX', default='.*')
arg_parser.add_argument('--show-steps', default=False, action='store_true')
arg_parser.add_argument('--ci', default=False, action='store_true')
arg_parser.add_argument('--rss-file', metavar='PATH', default='')
arg_parser.add_argument('--nop-dlclose', default=False, action='store_true')
arg_parser.add_argument('--no-sequential', default=False, action='store_true')
args_parsed = arg_parser.parse_args()
GENVM = Path(args_parsed.gen_vm)
FILE_RE = re.compile(args_parsed.filter)

if not GENVM.exists():
	print(f'genvm executable {GENVM} does not exist')
	exit(1)

import typing
import threading

MAX_WORKERS = max(1, (os.cpu_count() or 1) - 2)
print(f'concurrency is set to {MAX_WORKERS}')

RUN_SEMAPHORE = threading.BoundedSemaphore(MAX_WORKERS)


def unfold_conf(x: typing.Any, vars: dict[str, str]) -> typing.Any:
	if isinstance(x, str):
		return re.sub(r'\$\{[a-zA-Z\-_]+\}', lambda x: vars[x.group()[2:-1]], x)
	if isinstance(x, list):
		return [unfold_conf(x, vars) for x in x]
	if isinstance(x, dict):
		return {k: unfold_conf(v, vars) for k, v in x.items()}
	return x


def run(jsonnet_rel_path):
	jsonnet_path = dir.joinpath(jsonnet_rel_path)
	skipped = jsonnet_path.with_suffix('.skip')
	if skipped.exists():
		return {
			'category': 'skip',
		}
	jsonnet_conf = _jsonnet.evaluate_file(
		str(jsonnet_path), jpathdir=[str(script_dir.parent)]
	)
	jsonnet_conf = json.loads(jsonnet_conf)
	if not isinstance(jsonnet_conf, list):
		jsonnet_conf = [jsonnet_conf]

	jsonnet_conf = unfold_conf(jsonnet_conf, {'jsonnetDir': str(jsonnet_path.parent)})

	seq_tmp_dir = root_tmp_dir.joinpath(jsonnet_rel_path).with_suffix('')

	import shutil

	shutil.rmtree(seq_tmp_dir, ignore_errors=True)
	seq_tmp_dir.mkdir(exist_ok=True, parents=True)

	base_mock_storage = MockStorage()
	import base64

	for addr, code in jsonnet_conf[0]['accounts'].items():
		code = code.get('code')
		if code is None:
			continue
		addr = base64.b64decode(addr)
		if code.endswith('.wat'):
			out_path = seq_tmp_dir.joinpath(Path(code).with_suffix('.wasm').name)
			subprocess.run(['wat2wasm', '-o', out_path, code], check=True)
			code = out_path
		else:
			code = Path(code)
		code = Path(code).read_bytes()
		base_host.save_code_callback(
			addr, code, lambda a, b, c, d: base_mock_storage.write(Address(a), b, c, d)
		)
	empty_storage = seq_tmp_dir.joinpath('empty-storage.pickle')
	with open(empty_storage, 'wb') as f:
		pickle.dump(base_mock_storage, f)

	def step_to_run_config(i, single_conf_form_file, total_conf):
		single_conf_form_file = pickle.loads(pickle.dumps(single_conf_form_file))
		if total_conf == 1:
			my_tmp_dir = seq_tmp_dir
			suff = ''
		else:
			my_tmp_dir = seq_tmp_dir.joinpath(str(i))
			suff = f'.{i}'
		if i == 0:
			pre_storage = empty_storage
		else:
			pre_storage = seq_tmp_dir.joinpath(str(i - 1), 'storage.pickle')
		post_storage = my_tmp_dir.joinpath('storage.pickle')
		my_tmp_dir.mkdir(exist_ok=True, parents=True)

		if 'prepare' in single_conf_form_file:
			subprocess.run(
				[sys.executable, single_conf_form_file['prepare']],
				stdin=subprocess.DEVNULL,
				stdout=sys.stdout,
				stderr=sys.stderr,
				check=True,
			)
		for acc_val in single_conf_form_file['accounts'].values():
			code_path = acc_val.get('code', None)
			if code_path is None:
				continue
			if code_path.endswith('.wat'):
				out_path = my_tmp_dir.joinpath(Path(code_path).with_suffix('.wasm').name)
				subprocess.run(['wat2wasm', '-o', out_path, code_path], check=True)
				acc_val['code'] = str(out_path)

		calldata_bytes = calldata.encode(
			eval(
				single_conf_form_file['calldata'],
				globals(),
				single_conf_form_file['vars'].copy(),
			)
		)
		messages_path = my_tmp_dir.joinpath('messages.txt')
		# here tmp is used because of size limit for sock path
		mock_sock_path = Path(
			'/tmp', 'genvm-test', jsonnet_rel_path.with_suffix(f'.sock{suff}')
		)
		mock_sock_path.parent.mkdir(exist_ok=True, parents=True)
		host = MockHost(
			path=str(mock_sock_path),
			calldata=calldata_bytes,
			storage_path_post=post_storage,
			storage_path_pre=pre_storage,
			leader_nondet=single_conf_form_file.get('leader_nondet', None),
			messages_path=messages_path,
			balances={
				Address(k): v for k, v in single_conf_form_file.get('balances', {}).items()
			},
		)

		mock_host_path = my_tmp_dir.joinpath('mock-host.pickle')
		mock_host_path.write_bytes(pickle.dumps(host))
		return {
			'host': host,
			'message': single_conf_form_file['message'],
			'sync': single_conf_form_file.get('sync', False),
			'tmp_dir': my_tmp_dir,
			'expected_output': jsonnet_path.with_suffix(f'{suff}.stdout'),
			'suff': suff,
			'mock_host_path': mock_host_path,
			'messages_path': messages_path,
			'expected_messages_path': jsonnet_path.with_suffix(f'{suff}.msgs'),
			'deadline': single_conf_form_file.get('deadline', None),
		}

	run_configs = [
		step_to_run_config(i, conf_i, len(jsonnet_conf))
		for i, conf_i in enumerate(jsonnet_conf)
	]
	base = {}
	for config in run_configs:
		tmp_dir = config['tmp_dir']
		cmd = []
		# it is impossible to sigterm timed command (in an easy way)
		# cmd = [
		# '/usr/bin/time',
		# '-q',
		# '-f',
		# '<RSS>%M-%X</RSS>',
		# '--',
		# ]
		cmd.extend(
			[
				GENVM,
				'run',
				'--host',
				'unix://' + config['host'].path,
				'--message',
				json.dumps(config['message']),
				'--print=shrink',
				'--allow-latest',
			]
		)
		if config['sync']:
			cmd.append('--sync')
		steps = [
			[
				sys.executable,
				Path(__file__).absolute().parent.joinpath('mock_host.py'),
				config['mock_host_path'],
			],
			cmd,
		]
		with config['host'] as mock_host:
			_env = dict(os.environ)
			if args_parsed.nop_dlclose:
				_env['LD_PRELOAD'] = str(GENVM.parent.parent.parent.joinpath('fake-dlclose.so'))
				# _env["LD_DEBUG"] = "libs"

			try:
				res = asyncio.run(
					run_host_and_program(
						mock_host,
						cmd,
						env=_env,
						cwd=coverage_dir,
						exit_timeout=2,
						deadline=config['deadline'],
					)
				)
			except Exception as e:
				print(e.args)
				return {
					'category': 'fail',
					'steps': steps,
					'exception': 'internal error',
					'exc': e,
					**e.args[-1],
				}

		base = {
			'steps': steps,
			'stdout': res.stdout,
			'stderr': res.stderr,
			'genvm_log': res.genvm_log,
		}

		got_stdout_path = tmp_dir.joinpath('stdout.txt')
		got_stdout_path.parent.mkdir(parents=True, exist_ok=True)
		got_stdout_path.write_text(res.stdout)
		tmp_dir.joinpath('stderr.txt').write_text(res.stderr)
		tmp_dir.joinpath('genvm.log').write_text(res.genvm_log)

		exp_stdout_path = config['expected_output']
		if exp_stdout_path.exists():
			if exp_stdout_path.read_text() != res.stdout:
				return {
					'category': 'fail',
					'reason': f'stdout mismatch, see\n\tdiff {str(exp_stdout_path)} {str(got_stdout_path)}',
					**base,
				}
		else:
			exp_stdout_path.write_text(res.stdout)

		messages_path: Path = config['messages_path']
		expected_messages_path: Path = config['expected_messages_path']
		if messages_path.exists() != expected_messages_path.exists():
			return {
				'category': 'fail',
				'reason': f'messages do not exists\n\tdiff {messages_path} {expected_messages_path}',
				**base,
			}
		if messages_path.exists():
			got = messages_path.read_text()
			exp = expected_messages_path.read_text()
			if got != exp:
				return {
					'category': 'fail',
					'reason': f'messages differ\n\tdiff {messages_path} {expected_messages_path}',
					**base,
				}

	return {'category': 'pass', **base}


# semaphore should not be needed
# with thread pool executor
# but on CI some strange behavior happens
def run_with_semaphore(*args, **kwargs):
	with RUN_SEMAPHORE:
		return run(*args, **kwargs)


files = [x.relative_to(dir) for x in dir.glob('**/*.jsonnet')]
files = [x for x in files if FILE_RE.search(str(x)) is not None]
files.sort()


class COLORS:
	HEADER = '\033[95m'
	OKBLUE = '\033[94m'
	OKCYAN = '\033[96m'
	OKGREEN = '\033[92m'
	WARNING = '\033[93m'
	FAIL = '\033[91m'
	ENDC = '\033[0m'
	BOLD = '\033[1m'
	UNDERLINE = '\033[4m'


prnt_mutex = Lock()


def prnt(path, res):
	with prnt_mutex:
		print(f"{sign_by_category[res['category']]} {path}")
		if 'reason' in res:
			for l in map(lambda x: '\t' + x, res['reason'].split('\n')):
				print(l)
		if 'exc' in res:
			import traceback

			exc = res['exc']
			if not isinstance(exc, list):
				exc = [exc]
			for e in exc:
				st = traceback.format_exception(e)
				print(re.sub(r'^', '\t\t', ''.join(st), flags=re.MULTILINE))
		if res['category'] == 'fail' and 'steps' in res or args_parsed.show_steps:
			import shlex

			print('\tsteps to reproduce:')
			for line in res['steps']:
				print(f"\t\t{' '.join(map(lambda x: shlex.quote(str(x)), line))}")
		if res['category'] == 'fail':

			def print_lines(st):
				lines = st.splitlines()
				if args_parsed.ci:
					for l in lines:
						print(f'\t\t{l}')
				else:
					for l in lines[:10]:
						print(f'\t\t{l}')
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


with cfutures.ThreadPoolExecutor(MAX_WORKERS) as executor:
	categories = {
		'skip': 0,
		'pass': 0,
		'fail': [],
	}
	rss = []
	sign_by_category = {
		'skip': '⚠ ',
		'pass': f'{COLORS.OKGREEN}✓{COLORS.ENDC}',
		'fail': f'{COLORS.FAIL}✗{COLORS.ENDC}',
	}

	def process_result(path, res_getter):
		try:
			res = res_getter()
		except Exception as e:
			res = {
				'category': 'fail',
				'reason': str(e),
				'exc': e,
			}
		if res['category'] == 'fail':
			categories['fail'].append(str(path))
		else:
			categories[res['category']] += 1
		if (
			'stderr' in res
			and (m := re.search(r'<RSS>(\d+)-(\d+)</RSS>', res['stderr'])) is not None
		):
			rss.append((int(m.group(1)) - int(m.group(2))) * 1024)
			print(rss[-1], m)
		prnt(path, res)

	if len(files) > 0:
		# NOTE this is needed to cache wasm compilation result
		if args_parsed.no_sequential:
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
		for f in firsts:
			process_result(f, lambda: run(f))
		future2path = {executor.submit(run_with_semaphore, path): path for path in lasts}
		for future in cfutures.as_completed(future2path):
			path = future2path[future]
			process_result(future2path[future], lambda: future.result())
	import json

	print(f'coverage data is located at {coverage_dir} (if any)')
	print(json.dumps(categories))

	if args_parsed.rss_file != '':
		import plotille

		with open(args_parsed.rss_file, 'wt') as f:
			rss_mb = [x / (1024 * 1024) for x in rss]
			f.write('## RSS - Avg shared (in MB)\n```\n')
			f.write(plotille.hist(rss_mb))
			f.write('\n```\n')

	if len(categories['fail']) != 0:
		exit(1)
	exit(0)
