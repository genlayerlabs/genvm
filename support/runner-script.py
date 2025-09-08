#!/usr/bin/env python3

from pathlib import Path

import sys
import os
import typing
import json
import urllib.request
import urllib.parse
import hashlib

import hashlib

HASH_VALID_CHARS = '0123456789abcdfghijklmnpqrsvwxyz'

ORIGINAL_ENV = os.environ.copy()
if 'ORIGINAL_PATH' in ORIGINAL_ENV:
	os.environ['PATH'] = ORIGINAL_ENV['ORIGINAL_PATH']
if 'ORIGINAL_LD_LIBRARY_PATH' in ORIGINAL_ENV:
	os.environ['LD_LIBRARY_PATH'] = ORIGINAL_ENV['ORIGINAL_LD_LIBRARY_PATH']
else:
	os.environ['LD_LIBRARY_PATH'] = '/usr/local/lib:/usr/lib:/lib'


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


def check_bytes(data: bytes, hash: str) -> bool:
	digest = hashlib.sha256(data).digest()
	my_hash = digest_to_hash_id(digest)
	return my_hash == hash


def _sys_exit(code: int) -> typing.NoReturn:
	r = SystemExit()
	r.code = code
	raise r


def run_verify_file(args):
	path = Path(args.file)
	expected_hash: str | None = args.expected_hash
	if expected_hash is None:
		expected_hash = path.name
		expected_hash = expected_hash.removesuffix('.tar')

	if not all([x in HASH_VALID_CHARS for x in expected_hash]):
		print(f'invalid hash {expected_hash}', file=sys.stderr)
		_sys_exit(1)

	with open(path, 'rb') as f:
		got_digest = hashlib.file_digest(f, 'sha256').digest()
	digest_as_str = digest_to_hash_id(got_digest)
	if digest_as_str != expected_hash:
		print(f'hash mismatch\nexp: {expected_hash}\ngot: {digest_as_str}', file=sys.stderr)
		_sys_exit(1)


def _load_registry(file: str) -> dict[str, list[str]]:
	if file == '-':
		contents = json.load(sys.stdin)
	else:
		with open(file, 'r') as f:
			contents = json.load(f)

	if not isinstance(contents, dict):
		raise RuntimeError('expected dict for registry')

	ret: dict[str, list[str]] = {}

	for k, v in contents.items():
		if isinstance(v, str):
			ret[k] = [v]
		elif isinstance(v, list):
			if not all([isinstance(x, str) for x in v]):
				raise RuntimeError(f'registry value must be str | list[str] for {k}')
			ret[k] = v

	for v in ret.values():
		v.sort()

	return ret


def _object_gcs_path(name: str, hash: str) -> str:
	return f'genvm_runners/{name}/{hash}.tar'


def _download_single(name: str, hash: str) -> bytes:
	url = f'https://storage.googleapis.com/gh-af/{_object_gcs_path(name, hash)}'
	with urllib.request.urlopen(url) as f:
		return f.read()


def _nix_preload(cur_dst: Path, name: str, hash: str):
	if not args.nix_preload:
		return
	import subprocess

	print(f'info: preloading {cur_dst} into nix store as genvm_runner_{name}_{hash}')

	subprocess.run(
		[
			'nix',
			'store',
			'add',
			'--hash-algo',
			'sha256',
			'--mode',
			'flat',
			'--name',
			f'genvm_runner_{name}_{hash}',
			str(cur_dst),
		],
		check=True,
		text=True,
		capture_output=True,
		env=ORIGINAL_ENV,
	)
	print(f'info: done preloading {cur_dst} into nix store as genvm_runner_{name}_{hash}')


def run_download(args):
	registry = _load_registry(args.registry)

	dst = Path(args.dest)

	successful: dict[str, list[str]] = {}

	for name, hashes in registry.items():
		for hash in hashes:
			try:
				cur_dst = dst.joinpath(name, hash[:2], hash[2:] + '.tar')

				if cur_dst.exists():
					data = cur_dst.read_bytes()
					if check_bytes(data, hash):
						print(f'info: already exists {name}:{hash}, skipping')
						_nix_preload(cur_dst, name, hash)
						continue
					print(f'err: exists corrupted {name}:{hash}, removing')
					cur_dst.unlink()
				else:
					print(f'info: not found {cur_dst}')

				data = _download_single(name, hash)
				if not check_bytes(data, hash):
					raise ValueError('hash mismatch')

				cur_dst.parent.mkdir(parents=True, exist_ok=True)

				cur_dst.write_bytes(data)

				_nix_preload(cur_dst, name, hash)

				successful.setdefault(name, []).append(hash)
			except Exception as e:
				if args.allow_partial:
					print(f'warn: failed to download {name}:{hash}, {e}', file=sys.stderr)
				else:
					print(f'err: failed to download {name}:{hash}', file=sys.stderr)
					raise

	print(json.dumps({'downloaded': successful}, sort_keys=True))


def _upload_single(name: str, hash: str, contents: bytes, *, token: str):
	if not check_bytes(contents, hash):
		raise ValueError('hash mismatch')

	object_name = urllib.parse.quote_plus(_object_gcs_path(name, hash))

	upload_url = f'https://storage.googleapis.com/upload/storage/v1/b/gh-af/o?uploadType=media&name={object_name}'

	req = urllib.request.Request(
		url=upload_url,
		data=contents,
		method='POST',
		headers={
			'Authorization': f'Bearer {token}',
			'Content-Type': 'application/octet-stream',
		},
	)

	with urllib.request.urlopen(req) as resp:
		resp.read()


def run_upload(args):
	import subprocess

	proc = subprocess.run(
		['gcloud', 'auth', 'print-access-token'],
		check=True,
		text=True,
		capture_output=True,
		env=ORIGINAL_ENV,
	)
	token = proc.stdout.strip()

	registry = _load_registry(args.registry)

	root = Path(args.root)

	for name, hashes in registry.items():
		for hash in hashes:
			try:
				print(f'trying {name}:{hash} ...')
				data = root.joinpath(name, hash[:2], hash[2:] + '.tar').read_bytes()
				_upload_single(name, hash, data, token=token)
			except Exception as e:
				print(f'warn: failed to upload {name}:{hash}, {e}', file=sys.stderr)


def run_merge(args):
	all_regs: dict[str, set[str]] = {}

	for f in args.file:
		for name, hashes in _load_registry(f).items():
			all_regs.setdefault(name, set()).update(hashes)

	all_regs_list: dict[str, list[str]] = {}
	for k, v in all_regs.items():
		all_regs_list[k] = list(sorted(v))

	print(json.dumps(all_regs_list, sort_keys=True))


if __name__ == '__main__':
	import argparse

	parser = argparse.ArgumentParser('genvm-runners-registry')

	subparsers = parser.add_subparsers()

	verify_file_parser = subparsers.add_parser('verify-file')
	verify_file_parser.add_argument('file')
	verify_file_parser.add_argument('--expected-hash')
	verify_file_parser.set_defaults(func=run_verify_file)

	download_parser = subparsers.add_parser('download')
	download_parser.add_argument('--dest', default='.')
	download_parser.add_argument('--allow-partial', default=False, action='store_true')
	download_parser.add_argument('--nix-preload', default=False, action='store_true')
	download_parser.add_argument('--registry', required=True, metavar='FILE')
	download_parser.set_defaults(func=run_download)

	upload_file_parser = subparsers.add_parser('upload')
	upload_file_parser.add_argument('--root', default='.')
	upload_file_parser.add_argument('--registry', required=True, metavar='FILE')
	upload_file_parser.set_defaults(func=run_upload)

	merge_parser = subparsers.add_parser('merge-registries')
	merge_parser.add_argument('file', nargs='+')
	merge_parser.set_defaults(func=run_merge)

	args = parser.parse_args()
	if 'func' not in args:
		print(f'subcommand not given')
		parser.print_help()
		_sys_exit(1)
	args.func(args)
