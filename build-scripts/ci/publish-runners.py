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
import urllib.parse
import traceback

proc = subprocess.run(
	['gcloud', 'auth', 'print-access-token'], check=True, text=True, capture_output=True
)
token = proc.stdout.strip()

for id, hash in latest.items():
	print(f'{id}:{hash}')
	url = f'https://storage.googleapis.com/gh-af/genvm_runners/{id}/{hash}.tar'

	build_path = runners_path.joinpath(id, hash + '.tar')

	data = build_path.read_bytes()

	object_name = urllib.parse.quote_plus(f'genvm_runners/{id}/{hash}.tar')

	upload_url = f'https://storage.googleapis.com/upload/storage/v1/b/gh-af/o?uploadType=media&name={object_name}'

	req = urllib.request.Request(
		url=upload_url,
		data=data,
		method='POST',
		headers={
			'Authorization': f'Bearer {token}',
			'Content-Type': 'application/octet-stream',
		},
	)

	try:
		with urllib.request.urlopen(req) as resp:
			print(resp.read())
			print('ok')
	except KeyboardInterrupt:
		raise
	except BaseException:
		print('upload failed')
		traceback.print_exc()

	break
