#!/usr/bin/env python3

import sys

if len(sys.argv) != 2:
	print(f'invalid argv. expected [config], got {sys.argv[1:]}', file=sys.stderr)

import tarfile, zipfile
import json
import io
import hashlib
from pathlib import Path

with open(sys.argv[1]) as f:
	conf = json.load(f)

DEFAULT_TIME = (1980, 1, 1, 0, 0, 0)

fake_tar_io = io.BytesIO()
with tarfile.TarFile(
	fileobj=fake_tar_io, mode='w', format=tarfile.USTAR_FORMAT, encoding='utf-8'
) as inmem_tar_file:

	def add_file(name: str, contents: bytes, ctx={}):
		contents_hash = hashlib.sha3_256()
		contents_hash.update(contents)
		print(f'\tADDING {contents_hash.digest().hex()} {name}\t{ctx}')
		info = tarfile.TarInfo()
		info.name = name
		info.size = len(contents)
		inmem_tar_file.addfile(info, io.BytesIO(contents))

	for file_conf in conf['files']:
		if 'include' in file_conf:
			if file_conf['include'].endswith('.zip'):
				with zipfile.ZipFile(file_conf['include'], mode='r') as incl_zip:
					for f in sorted(incl_zip.filelist, key=lambda k: k.filename):
						contents = incl_zip.read(f)
						add_file(f.filename, contents)
			else:
				with tarfile.open(
					file_conf['include'], mode='r|', format=tarfile.USTAR_FORMAT
				) as incl_tar:
					for f in sorted(incl_tar, key=lambda x: x.name):
						data = incl_tar.extractfile(f)
						assert data is not None
						contents = data.read(f.size)
						add_file(f.name, contents)
			continue
		read_from = file_conf['read_from']
		with open(read_from, 'rb') as f:
			contents = f.read()
		path = file_conf['path']
		add_file(path, contents, {'read_from': read_from})
fake_tar_io.flush()

tar_contents = fake_tar_io.getvalue()

contents_hash = hashlib.sha3_256()
contents_hash.update(tar_contents)
import base64

contents_hash = str(base64.b32encode(contents_hash.digest()), encoding='ascii')

contents_hash = contents_hash.replace('=', '')

print(f'CREATING {contents_hash}.tar')

assert conf['expected_hash'] is not None

if conf['expected_hash'] != 'test' and conf['expected_hash'] != contents_hash:
	raise Exception(
		f'hashes diverge for {conf["out_dir"]}\nexp: {conf["expected_hash"]}\ngot: {contents_hash}\nIf it is desired, update hash at yabuild-default-conf.rb'
	)

contents_hash = conf['expected_hash']

out_dir = Path(conf['out_dir'])
out_dir.mkdir(parents=True, exist_ok=True)
out_name = out_dir.joinpath(f'{contents_hash}.tar')
with open(out_name, 'wb') as f:
	f.write(tar_contents)
