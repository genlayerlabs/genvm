import argparse

arg_parser = argparse.ArgumentParser('genvm-make-runner')
arg_parser.add_argument(
	'--expected-hash',
	metavar='HASH',
	required=True,
)

arg_parser.add_argument(
	'--out-dir',
	metavar='DIR',
	required=True,
)

arg_parser.add_argument(
	'--src-dir',
	metavar='DIR',
	required=True,
)

arg_parser.add_argument(
	'--config',
	metavar='PATH',
	default='#all',
)

arg_parser.add_argument(
	'--runner-json',
	metavar='JSON',
	default='#none',
)

args_parsed = arg_parser.parse_args()

import sys
import typing
import tarfile, zipfile
import json
import io
import hashlib
from pathlib import Path

if args_parsed.config == '#all':
	read_files_conf = [
		{
			'path': '',
			'read_from': './',
		}
	]
elif args_parsed.config == '#none':
	read_files_conf = []
else:
	read_files_conf = json.load(args_parsed.config)

src_dir = Path(args_parsed.src_dir)

DEFAULT_TIME = (1980, 1, 1, 0, 0, 0)

all_files: dict[str, bytes] = {}


def add_file(name: str, contents: bytes):
	if name in all_files:
		raise KeyError('EEXISTS')
	if name.endswith('/'):
		return
	if name == '':
		raise Exception('empty name')

	if name == 'runner.json':
		new_contents = (
			json.dumps(json.loads(contents), separators=(',', ':')) + '\n'
		).encode('utf-8')
		if new_contents != contents:
			print(f'minified json old: {len(contents)} new: {len(new_contents)}')
		contents = new_contents

	all_files[name] = contents


def add_many(files: typing.Iterable[tuple[str, bytes]], prefix: str = ''):
	for k, v in files:
		add_file(prefix + k, v)


if args_parsed.runner_json != '#none':
	add_file('runner.json', args_parsed.runner_json.encode('utf-8'))

for file_conf in read_files_conf:
	if 'include' in file_conf:
		read_target_path = src_dir.joinpath(file_conf['include'])
		if file_conf['include'].endswith('.zip'):
			with zipfile.ZipFile(read_target_path, mode='r') as incl_zip:
				add_many([(f.filename, incl_zip.read(f)) for f in incl_zip.filelist])
		else:
			with tarfile.open(
				read_target_path, mode='r|', format=tarfile.USTAR_FORMAT
			) as incl_tar:
				add_many([(f.name, incl_tar.extractfile(f).read(f.size)) for f in incl_tar])
	else:
		read_from = file_conf['read_from']
		base_path = src_dir.joinpath(read_from)
		if read_from.endswith('/'):
			add_many(
				[
					(str(p.relative_to(base_path)), p.read_bytes())
					for p in Path(base_path).glob('**/*')
					if p.is_file()
				],
				file_conf['path'],
			)
		else:
			add_file(file_conf['path'], base_path.read_bytes())

assert len(all_files) != 0

fake_tar_io = io.BytesIO()
with tarfile.TarFile(
	fileobj=fake_tar_io, mode='w', format=tarfile.USTAR_FORMAT, encoding='utf-8'
) as inmem_tar_file:
	for name, contents in sorted(all_files.items(), key=lambda x: x[0]):
		info = tarfile.TarInfo()
		info.name = name
		info.size = len(contents)
		inmem_tar_file.addfile(info, io.BytesIO(contents))

fake_tar_io.flush()
tar_contents = fake_tar_io.getvalue()

contents_hash = hashlib.sha3_256()
contents_hash.update(tar_contents)
import base64

contents_hash = str(base64.b32encode(contents_hash.digest()), encoding='ascii')

contents_hash = contents_hash.replace('=', '')

print(f'CREATING {contents_hash}.tar which has {len(all_files)} files')

if args_parsed.expected_hash != 'test' and args_parsed.expected_hash != contents_hash:
	raise Exception(
		f'hashes diverge for {args_parsed.out_dir}\nexp: {args_parsed.expected_hash}\ngot: {contents_hash}\nIf it is desired, update hash at yabuild-default-conf.rb'
	)

contents_hash = args_parsed.expected_hash

out_dir = Path(args_parsed.out_dir)
out_dir.mkdir(parents=True, exist_ok=True)
out_name = out_dir.joinpath(f'{contents_hash}.tar')
with open(out_name, 'wb') as f:
	f.write(tar_contents)
