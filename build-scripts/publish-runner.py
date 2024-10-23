#!/usr/bin/env python3

import sys

if len(sys.argv) != 2:
	print(f'invalid argv. expected [config], got {sys.argv[1:]}', file=sys.stderr)

import zipfile
import json
import io
import hashlib
from pathlib import Path

with open(sys.argv[1]) as f:
	conf = json.load(f)

DEFAULT_TIME = (1980, 1, 1, 0, 0, 0)

fake_zip = io.BytesIO()
with zipfile.ZipFile(fake_zip, mode='w', compression=zipfile.ZIP_STORED) as zip_file:

	def add_file(name, contents, ctx={}):
		contents_hash = hashlib.sha3_256()
		contents_hash.update(contents)
		print(f'\tADDING {f} {contents_hash.digest()} {ctx}')
		zip_file.writestr(zipfile.ZipInfo(name, date_time=DEFAULT_TIME), contents)

	for file_conf in conf['files']:
		if 'include' in file_conf:
			with zipfile.ZipFile(file_conf['include']) as incl_zip:
				files = incl_zip.namelist()
				files.sort()
				for f in files:
					info = incl_zip.getinfo(f)
					if info.is_dir():
						pass  # zip_file.mkdir(zipfile.ZipInfo(f, DEFAULT_TIME), 444)
					else:
						add_file(f, incl_zip.read(f))
			continue
		read_from = file_conf['read_from']
		with open(read_from, 'rb') as f:
			contents = f.read()
		path = file_conf['path']
		add_file(path, contents, {'read_from': read_from})
fake_zip.flush()

zip_contents = fake_zip.getvalue()

contents_hash = hashlib.sha3_512()
contents_hash.update(zip_contents)
import base64

contents_hash = str(base64.b32encode(contents_hash.digest()), encoding='ascii')

print(f'CREATING {contents_hash}.zip')

if conf['expected_hash'] is not None and conf['expected_hash'] != contents_hash:
	raise Exception(
		f'hashes diverge for {conf["out_dir"]}\nexp: {conf["expected_hash"]}\ngot: {contents_hash}\nIf it is desired, update hash at yabuild-default-conf.rb'
	)

out_dir = Path(conf['out_dir'])
out_dir.mkdir(parents=True, exist_ok=True)
out_name = out_dir.joinpath(f'{contents_hash}.zip')
with open(out_name, 'wb') as f:
	f.write(zip_contents)

with open(conf['fake_out'], 'wt') as f:
	f.write(contents_hash)

if conf['create_test_runner']:
	out_dir.joinpath('test.zip').unlink(missing_ok=True)
	out_dir.joinpath('test.zip').symlink_to(out_name.relative_to(out_dir))
