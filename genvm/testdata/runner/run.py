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
import base64
import pickle

import http.server as httpserv

script_dir = Path(__file__).parent.absolute()
root_dir = script_dir
while not root_dir.joinpath('.genvm-monorepo-root').exists():
	root_dir = root_dir.parent

http_dir = str(script_dir.parent.joinpath('http').absolute())

sys.path.append(str(root_dir.joinpath('sdk-python', 'py')))

import genlayer.calldata as calldata
from genlayer.types import Address

class MyHTTPHandler(httpserv.SimpleHTTPRequestHandler):
	def __init__(self, *args, **kwargs):
		httpserv.SimpleHTTPRequestHandler.__init__(self, *args, **kwargs, directory=http_dir)

def run_serv():
	serv = httpserv.HTTPServer(('127.0.0.1', 4242), MyHTTPHandler)
	serv.serve_forever()

http_thread = threading.Thread(target=run_serv, daemon=True)
http_thread.start()

dir = script_dir.parent.joinpath('cases')
tmp_dir = root_dir.joinpath('build', 'genvm-testdata-out')

arg_parser = argparse.ArgumentParser("genvm-test-runner")
arg_parser.add_argument('--mock-gen-vm', metavar='EXE', default=str(Path(os.getenv("GENVM", root_dir.joinpath('build', 'out', 'bin', 'genvm-mock')))))
arg_parser.add_argument('--filter', metavar='REGEX', default='.*')
args_parsed = arg_parser.parse_args()
GENVM = Path(args_parsed.mock_gen_vm)
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

def run(path0):
	path = dir.joinpath(path0)
	skipped = path.with_suffix('.skip')
	if skipped.exists():
		return {
			"category": "skip",
		}
	conf = _jsonnet.evaluate_file(str(path))
	conf = json.loads(conf)
	if not isinstance(conf, list):
		conf = [conf]
	storage_path = tmp_dir.joinpath(path0).with_suffix(f'.storage')
	storage_path.unlink(missing_ok=True)
	steps = [
		["rm", str(storage_path)]
	]
	def map_conf(i, conf, total_conf):
		conf = pickle.loads(pickle.dumps(conf))
		if total_conf == 1:
			suff = ''
		else:
			suff = f'.{i}'
		conf["vars"]["jsonnetDir"] = str(path.parent)
		eval_vars = conf["vars"].copy()
		new_calldata_obj = eval(conf["calldata"], globals(), eval_vars)
		conf["calldata"] = str(base64.b64encode(calldata.encode(new_calldata_obj)), 'ascii')
		conf["storage_file_path"] = str(storage_path)
		conf = unfold_conf(conf, conf["vars"])
		conf_path = tmp_dir.joinpath(path0).with_suffix(f'{suff}.json')
		for acc_val in conf["accounts"].values():
			code_path = acc_val.get("code", None)
			if code_path is None:
				continue
			if code_path.endswith('.wat'):
				out_path = tmp_dir.joinpath(Path(code_path).with_suffix(".wasm").relative_to(dir))
				subprocess.run(['wat2wasm', '-o', out_path, code_path], check=True)
				acc_val["code"] = str(out_path)
			pass
		conf_path.parent.mkdir(parents=True, exist_ok=True)
		with open(conf_path, 'wt') as f:
			json.dump(conf, f)
		return conf_path
	conf_paths = [
		map_conf(i, conf_i, len(conf))
		for i, conf_i in enumerate(conf)
	]
	for conf_path in conf_paths:
		cmd = [GENVM, '--config', conf_path, '--shrink-error']
		steps.append(cmd)
		res = subprocess.run(cmd, check=False, text=True, capture_output=True)
		base = {
			"steps": pickle.loads(pickle.dumps(steps))
		}
		if res.returncode != 0:
			return {
				"category": "fail",
				"reason": f"return code is {res.returncode}\n=== stdout ===\n{res.stdout}\n=== stderr ===\n{res.stderr}",
				**base
			}
		res_path = conf_path.with_suffix('.stdout')
		stdout = path.parent.joinpath(res_path.name)
		if stdout.exists():
			res_path.parent.mkdir(parents=True, exist_ok=True)
			res_path.write_text(res.stdout)

			if stdout.read_text() != res.stdout:
				return {
					"category": "fail",
					"reason": f"stdout mismatch, see\ndiff {str(stdout)} {str(res_path)}",
					**base
				}
		else:
			stdout.write_text(res.stdout)
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
		if res['category'] == "fail" and "steps" in res:
			import shlex
			print("\tsteps to reproduce:")
			for line in res["steps"]:
				print(f"\t\t{' '.join(map(lambda x: shlex.quote(str(x)), line))}")

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
			}
		if res["category"] == "fail":
			categories["fail"].append(str(path))
		else:
			categories[res["category"]] += 1
		prnt(path, res)
	if len(files) > 0:
		# NOTE this is needed to cache wasm compilation result
		first, *files = files
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
