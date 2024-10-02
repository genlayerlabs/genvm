#!/bin/env python3
from pathlib import Path
import concurrent.futures as cfutures
import os
import subprocess
import _jsonnet
import json
import threading
from threading import Lock
import argparse
import re
import sys
import pickle
import time

import http.server as httpserv

MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
script_dir = Path(__file__).parent.absolute()
http_dir = str(script_dir.parent.joinpath('http').absolute())

root_dir = script_dir
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent
MONOREPO_CONF = json.loads(root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text())

sys.path.append(str(root_dir.joinpath(*MONOREPO_CONF["py-std"])))

from genlayer.py import calldata
from genlayer.py.types import Address
from mock_host import MockHost, MockStorage

class MyHTTPHandler(httpserv.SimpleHTTPRequestHandler):
	def __init__(self, *args, **kwargs):
		httpserv.SimpleHTTPRequestHandler.__init__(self, *args, **kwargs, directory=http_dir)

def run_serv():
	serv = httpserv.HTTPServer(('127.0.0.1', 4242), MyHTTPHandler)
	serv.serve_forever()

http_thread = threading.Thread(target=run_serv, daemon=True)
http_thread.start()

dir = script_dir.parent.joinpath('cases')
root_tmp_dir = root_dir.joinpath('build', 'genvm-testdata-out')

arg_parser = argparse.ArgumentParser("genvm-test-runner")
arg_parser.add_argument('--gen-vm', metavar='EXE', default=str(Path(os.getenv("GENVM", root_dir.joinpath('build', 'out', 'bin', 'genvm')))))
arg_parser.add_argument('--filter', metavar='REGEX', default='.*')
arg_parser.add_argument('--show-steps', default=False, action='store_true')
arg_parser.add_argument('--nop-dlclose', default=False, action='store_true')
args_parsed = arg_parser.parse_args()
GENVM = Path(args_parsed.gen_vm)
FILE_RE = re.compile(args_parsed.filter)

if not GENVM.exists():
	print(f'genvm executable {GENVM} does not exist')
	exit(1)

import typing
def unfold_conf(x: typing.Any, vars: dict[str, str]) -> typing.Any:
	if isinstance(x, str):
		return re.sub(r"\$\{[a-zA-Z\-_]+\}", lambda x: vars[x.group()[2:-1]], x)
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
			"category": "skip",
		}
	jsonnet_conf = _jsonnet.evaluate_file(str(jsonnet_path))
	jsonnet_conf = json.loads(jsonnet_conf)
	if not isinstance(jsonnet_conf, list):
		jsonnet_conf = [jsonnet_conf]
	seq_tmp_dir = root_tmp_dir.joinpath(jsonnet_rel_path).with_suffix('')

	import shutil
	shutil.rmtree(seq_tmp_dir, ignore_errors=True)
	seq_tmp_dir.mkdir(exist_ok=True, parents=True)

	empty_storage = seq_tmp_dir.joinpath('empty-storage.pickle')
	with open(empty_storage, 'wb') as f:
		pickle.dump(MockStorage(), f)

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

		single_conf_form_file["vars"]["jsonnetDir"] = str(jsonnet_path.parent)

		single_conf_form_file = unfold_conf(single_conf_form_file, single_conf_form_file["vars"])
		for acc_val in single_conf_form_file["accounts"].values():
			code_path = acc_val.get("code", None)
			if code_path is None:
				continue
			if code_path.endswith('.wat'):
				out_path = my_tmp_dir.joinpath(Path(code_path).with_suffix(".wasm").name)
				subprocess.run(['wat2wasm', '-o', out_path, code_path], check=True)
				acc_val["code"] = str(out_path)

		calldata_bytes = calldata.encode(
			eval(
				single_conf_form_file["calldata"],
				globals(),
				single_conf_form_file["vars"].copy()
			)
		)
		# here tmp is used because of size limit for sock path
		mock_sock_path = Path('/tmp', 'genvm-test', jsonnet_rel_path.with_suffix(f'.sock{suff}'))
		mock_sock_path.parent.mkdir(exist_ok=True, parents=True)
		host = MockHost(
			path=str(mock_sock_path),
			calldata=calldata_bytes,
			codes={
				Address(k): v
				for k, v in single_conf_form_file["accounts"].items()
			},
			storage_path_post=post_storage,
			storage_path_pre=pre_storage,
			leader_nondet=single_conf_form_file.get('leader_nondet', None)
		)
		mock_host_path = my_tmp_dir.joinpath('mock-host.pickle')
		mock_host_path.write_bytes(pickle.dumps(host))
		return {
			"host": host,
			"message": single_conf_form_file["message"],
			"tmp_dir": my_tmp_dir,
			"expected_output": jsonnet_path.with_suffix(f'{suff}.stdout'),
			"suff": suff,
			"mock_host_path": mock_host_path,
		}
	run_configs = [
		step_to_run_config(i, conf_i, len(jsonnet_conf))
		for i, conf_i in enumerate(jsonnet_conf)
	]
	for config in run_configs:
		tmp_dir = config["tmp_dir"]
		cmd = [GENVM, '--host', "unix://" + config["host"].path, '--message', json.dumps(config["message"]), '--print=shrink']
		steps = [
			[sys.executable, '-m', 'http.server', '--directory', http_dir, '4242'],
			[sys.executable, Path(__file__).absolute().parent.joinpath('mock_host.py'), config["mock_host_path"]],
			cmd
		]
		with config["host"] as mock_host:
			while not mock_host.created:
				time.sleep(0.05)
			_env = dict(os.environ)
			if args_parsed.nop_dlclose:
				_env["LD_PRELOAD"] = GENVM.parent.parent.parent.joinpath('fake-dlclose.so')
				#_env["LD_DEBUG"] = "libs"
			res = subprocess.run(cmd, check=False, text=True, capture_output=True, env=_env)
		base = {
			"steps": steps,
			"stdout": res.stdout,
			"stderr": res.stderr,
		}

		got_stdout_path = tmp_dir.joinpath('stdout.txt')
		got_stdout_path.parent.mkdir(parents=True, exist_ok=True)
		got_stdout_path.write_text(res.stdout)
		tmp_dir.joinpath('stderr.txt').write_text(res.stderr)

		if res.returncode != 0:
			return {
				"category": "fail",
				"reason": f"return code is {res.returncode}",
				**base
			}

		exp_stdout_path = config["expected_output"]
		if exp_stdout_path.exists():
			if exp_stdout_path.read_text() != res.stdout:
				return {
					"category": "fail",
					"reason": f"stdout mismatch, see\n\tdiff {str(exp_stdout_path)} {str(got_stdout_path)}",
					**base
				}
		else:
			exp_stdout_path.write_text(res.stdout)
	return {
		"category": "pass",
		**base
	}

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
		if "reason" in res:
			for l in map(lambda x: '\t' + x, res["reason"].split('\n')):
				print(l)
		if "exc" in res and not isinstance(res["exc"], subprocess.CalledProcessError):
			import traceback
			traceback.print_exception(res["exc"])
		if res['category'] == "fail" and "steps" in res or args_parsed.show_steps:
			import shlex
			print("\tsteps to reproduce:")
			for line in res["steps"]:
				print(f"\t\t{' '.join(map(lambda x: shlex.quote(str(x)), line))}")
		if res['category'] == "fail":
			def print_lines(st):
				lines = st.splitlines()
				for l in lines[:10]:
					print(f"\t\t{l}")
				if len(lines) >= 10:
					print("\t...")
			if "stdout" in res:
				print("\t=== stdout ===")
				print_lines(res["stdout"])
			if "stderr" in res:
				print("\t=== stderr ===")
				print_lines(res["stderr"])

with cfutures.ThreadPoolExecutor(max_workers=(os.cpu_count() or 1)) as executor:
	categories = {
		"skip": 0,
		"pass": 0,
		"fail": [],
	}
	sign_by_category = {
		"skip": "⚠ ",
		"pass": f"{COLORS.OKGREEN}✓{COLORS.ENDC}",
		"fail": f"{COLORS.FAIL}✗{COLORS.ENDC}",
	}
	def process_result(path, res_getter):
		try:
			res = res_getter()
		except Exception as e:
			res = {
				"category": "fail",
				"reason": str(e),
				"exc": e,
			}
		if res["category"] == "fail":
			categories["fail"].append(str(path))
		else:
			categories[res["category"]] += 1
		prnt(path, res)
	if len(files) > 0:
		# NOTE this is needed to cache wasm compilation result
		first, *files = files
		print('running the first test, it can take a while..')
		process_result(first, lambda: run(first))
		future2path = {executor.submit(run, path): path for path in files}
		for future in cfutures.as_completed(future2path):
			path = future2path[future]
			process_result(future2path[future], lambda: future.result())
	import json
	print(json.dumps(categories))
	if len(categories["fail"]) != 0:
		exit(1)
	exit(0)
