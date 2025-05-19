#!/usr/bin/env python3

import subprocess
import sys
import json
import hashlib

from pathlib import Path

root_dir = Path(__file__)
MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent

runners_path = root_dir.joinpath(*'build/out/share/lib/genvm/runners/'.split('/'))

latest_nix = root_dir.joinpath('runners', 'latest.nix')

import os

proc = subprocess.run(
	['nix', 'eval', '--read-only', '--show-trace', '--json', '--file', latest_nix],
	check=True,
	capture_output=True,
	text=True,
)

latest: dict[str, str] = json.loads(proc.stdout)

print(f'{latest}')


def digest_to_hash_id(got_hash: bytes) -> str:
	chars = '0123456789abcdfghijklmnpqrsvwxyz'

	bytes_count = len(got_hash)
	base32_len = (bytes_count * 8 - 1) // 5 + 1

	my_hash_arr = []
	for n in range(base32_len - 1, -1, -1):
		b = n * 5
		i = b // 8
		j = b % 8
		c = (got_hash[i] >> j) | (0 if i >= bytes_count - 1 else got_hash[i + 1] << (8 - j))
		my_hash_arr.append(chars[c & 0x1F])

	return ''.join(my_hash_arr)


import urllib.request

for id, hash in latest.items():
	print(f'{id}:{hash}')

	url = f'https://storage.googleapis.com/gh-af/genvm_runners/{id}-{hash}.tar'

	try:
		with urllib.request.urlopen(url) as f:
			contents = f.read()
	except BaseException:
		print('not found')
		continue

	hash_sha = hashlib.sha256(contents).digest()
	my_hash = digest_to_hash_id(hash_sha)

	if my_hash != hash:
		print(f'HASH MISMATCH FOR {id}\ngot: {my_hash}\nexp: {hash}', file=sys.stderr)
		continue

	build_path = runners_path.joinpath(id, hash + '.tar')
	build_path.parent.mkdir(parents=True, exist_ok=True)
	build_path.write_bytes(contents)

	del contents

	subprocess.run(
		[
			'nix',
			'store',
			'add',
			'--hash-algo',
			'sha256',
			'--mode',
			'nar',
			'--name',
			f'genvm_runner_{id}_{hash}',
			build_path,
		],
		check=True,
		text=True,
	)
