#!/usr/bin/env python3

import sys
import re

skip_crt_re = re.compile(r'lib/self-contained/crt[a-z0-9]*\.o$')

args = sys.argv[1:]
from pathlib import Path

root_dir = Path(__file__)
MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent


def unfold_target(t: str) -> str:
	match t:
		case 'aarch64-unknown-linux-gnu':
			return 'aarch64-linux-gnu'
		case 'aarch64-unknown-linux-musl':
			return 'aarch64-linux-musl'
		case 'arm64-apple-darwin' | 'aarch64-apple-darwin' | 'arm64-apple-macosx':
			return 'aarch64-macos-none'
		case 'x86_64-unknown-linux-gnu':
			return 'x86_64-linux-gnu'
		case 'x86_64-unknown-linux-musl':
			return 'x86_64-linux-musl'
		case t:
			return t


libs: set[str] = set()
lib_dirs: set[Path] = set()


def mp(a: str) -> list[str]:
	if a.startswith('--target='):
		return ['-target', unfold_target(a[len('--target=') :])]
	if unfold_target(a) != a:
		return [unfold_target(a)]
	if a.endswith('.rlib'):
		p = Path(a)
		new_p = p.with_suffix('.rlib.a')
		new_p.write_bytes(p.read_bytes())
		return [str(new_p)]
	if a.startswith('-l'):
		libs.add(a[2:])
		return []
	if a in ['-nodefaultlibs']:
		return []
	if a.startswith('-l'):
		libs.add(a[2:])
		return []
	if skip_crt_re.search(a):
		return []
	return [a]


new_args = sum([mp(arg) for arg in args], [])

if len(libs) != 0:
	libs.discard('gcc_eh')
	libs.discard('gcc')
	libs.discard('gcc_s')
	libs.discard('c')
	libs.discard('m')

	libs.add('unwind')

	new_args.append('-static')
	new_args += map(lambda x: f'-l{x}', libs)

if '-target' not in new_args:
	import json

	conf = json.loads(root_dir.joinpath('build', 'config.json').read_text())
	trg = unfold_target(conf['executor_target'])
	if trg is not None:
		new_args[0:0] = ['-target', trg]

import subprocess

print(new_args)

subprocess.run(
	[
		root_dir.joinpath('tools', 'downloaded', 'zig', 'zig'),
		'cc',
		'-v',
		*new_args,
	],
	check=True,
	text=True,
	stdout=sys.stdout,
	stderr=sys.stderr,
)
