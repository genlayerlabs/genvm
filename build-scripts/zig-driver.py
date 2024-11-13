#!/usr/bin/env python3

import sys

args = sys.argv[1:]
from pathlib import Path

root_dir = Path(__file__)
MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent


def mp(a: str) -> list[str]:
	if a == '--target=aarch64-unknown-linux-gnu':
		return ['-target', 'aarch64-linux-gnu']
	if a.endswith('.rlib'):
		p = Path(a)
		new_p = p.with_suffix('.rlib.a')
		new_p.write_bytes(p.read_bytes())
		return [str(new_p)]
	return [a]


new_args = sum([mp(arg) for arg in args], [])

import subprocess

subprocess.run(
	[
		root_dir.joinpath('tools', 'downloaded', 'zig', 'zig'),
		'cc',
		'-v',
		'-target',
		'aarch64-linux-gnu',
		*new_args,
	],
	check=True,
	text=True,
	stdout=sys.stdout,
	stderr=sys.stderr,
)
