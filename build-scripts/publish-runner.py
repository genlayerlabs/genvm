#!/usr/bin/env python3

import sys

if len(sys.argv) != 2:
	print(f'invalid argv. expected [config], got {sys.argv[1:]}', file=sys.stderr)

import zipfile
import json
import io
import hashlib
from pathlib import Path

conf = json.loads(sys.argv[1])

fake_zip = io.BytesIO()
with zipfile.ZipFile(fake_zip, mode="w", compression=zipfile.ZIP_STORED) as zip_file:
	for file_conf in conf['files']:
		with open(file_conf['read_from'], 'rb') as f:
			contents = f.read()
		path = file_conf['path']
		info = zipfile.ZipInfo(path, date_time=(1980, 1, 1, 0, 0, 0))
		zip_file.writestr(info, contents)
fake_zip.flush()

zip_contents = fake_zip.getvalue()

contents_hash = hashlib.sha3_512()
contents_hash.update(zip_contents)
import base64
contents_hash = str(base64.b32encode(contents_hash.digest()), encoding='ascii')

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
