#!/bin/env python3
from pathlib import Path
import concurrent.futures as cfutures
import os
import subprocess
import _jsonnet
import json
from threading import Lock

dir = Path(__file__).parent.parent.joinpath('cases')
tmp_dir = Path(__file__).parent.parent.parent.joinpath('target', 'testdata-out')

GENVM = Path(os.getenv("GENVM", Path(__file__).parent.parent.parent.joinpath('target', 'debug', 'genvm-mock')))

if not GENVM.exists():
	print(f'genvm executable {GENVM} does not exist')
	exit(1)

def run(path0):
	path = dir.joinpath(path0)
	skipped = path.with_suffix('.skip')
	if skipped.exists():
		return {
			"category": "skip",
		}
	conf = _jsonnet.evaluate_file(str(path))
	conf = json.loads(conf)
	conf["vars"]["jsonnetDir"] = str(path.parent)
	conf_path = tmp_dir.joinpath(path0).with_suffix('.json')
	with open(conf_path, 'wt') as f:
		json.dump(conf, f)
	res = subprocess.run([GENVM, '--config', conf_path, '--shrink-error'], check=False, text=True, capture_output=True)
	if res.returncode != 0:
		return {
			"category": "fail",
			"reason": f"return code is {res.returncode}\n=== stdout ===\n{res.stdout}\n=== stderr ===\n{res.stderr}"
		}
	stdout = path.with_suffix('.stdout')
	if stdout.exists():
		res_path = tmp_dir.joinpath(path0).with_suffix('.stdout')
		res_path.parent.mkdir(parents=True, exist_ok=True)
		res_path.write_text(res.stdout)

		if stdout.read_text() != res.stdout:
			return {
				"category": "fail",
				"reason": f"stdout mismatch, see {str(res_path)}"
			}
	else:
		stdout.write_text(res.stdout)
	return {
		"category": "pass",
	}

files = [x.relative_to(dir) for x in dir.glob('**/*.jsonnet')]
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

with cfutures.ThreadPoolExecutor(max_workers=8) as executor:
	future2path = {executor.submit(run, path): path for path in files}
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
	for future in cfutures.as_completed(future2path):
		path = future2path[future]
		try:
			res = future.result()
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
	import json
	print(json.dumps(categories))
	if len(categories["fail"]) != 0:
		exit(1)
