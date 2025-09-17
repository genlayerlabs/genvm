#!/usr/bin/env python3

import subprocess, json, sys
from pathlib import Path

proc = subprocess.run(
	['git', 'remote', 'get-url', 'origin'],
	check=True,
	capture_output=True,
	text=True,
)

origin_url = proc.stdout.strip()

proc = subprocess.run(
	['git', 'ls-remote', '--tags', origin_url],
	check=True,
	capture_output=True,
	text=True,
)

commit2tag = {}

import re

bad_id = re.compile(r'[^0-9a-z._\-A-Z]')

for line in proc.stdout.splitlines():
	line = line.strip()
	comm_rev = line.split()
	if len(comm_rev) != 2:
		continue
	commit, rev = comm_rev

	rev = rev.removeprefix('refs/tags/')

	commit2tag[commit] = bad_id.sub('_', rev)

import os

cwd = Path(os.getcwd())

current_commit = subprocess.run(
	['git', 'rev-parse', 'HEAD'],
	check=True,
	capture_output=True,
	text=True,
)

current_commit = current_commit.stdout.strip()

eval_file = Path(__file__).parent.parent.joinpath('docs.nix')

build_config = json.loads(
	Path(__file__).parent.parent.parent.joinpath('flake-config.json').read_text()
)
build_config['head-revision'] = current_commit
commit2tag[current_commit] = build_config['executor-version']

print(commit2tag)
print(build_config)

proc = subprocess.run(
	[
		'nix',
		'eval',
		'--verbose',
		'--impure',
		'--read-only',
		'--show-trace',
		'--json',
		'--file',
		str(eval_file.relative_to(cwd)),
		'--apply',
		'f: f { commitToTagStr = "'
		+ json.dumps(commit2tag).replace('"', '\\"')
		+ '"; build-config-str = "'
		+ json.dumps(build_config).replace('"', '\\"')
		+ '"; }',
	],
	check=True,
	capture_output=True,
	text=True,
)

res = json.loads(proc.stdout)

Path(sys.argv[1]).write_text(json.dumps(res, indent=2))
