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

for line in proc.stdout.splitlines():
	line = line.strip()
	comm_rev = line.split()
	if len(comm_rev) != 2:
		continue
	commit, rev = comm_rev

	rev = rev.removeprefix('refs/tags/')

	commit2tag[commit] = rev

print(commit2tag)

import os

cwd = Path(os.getcwd())

current_commit = subprocess.run(
	['git', 'rev-parse', 'HEAD'],
	check=True,
	capture_output=True,
	text=True,
)

eval_file = Path(__file__).parent.parent.joinpath('docs.nix')

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
		'f: f { currentCommit = "'
		+ current_commit.stdout.strip()
		+ '"; commitToTagStr = "'
		+ json.dumps(commit2tag).replace('"', '\\"')
		+ '"; }',
	],
	check=True,
	capture_output=True,
	text=True,
)

res = json.loads(proc.stdout)

Path(sys.argv[1]).write_text(json.dumps(res, indent=2))
