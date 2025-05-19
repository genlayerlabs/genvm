import os
import tarfile
import json
import io
from pathlib import Path

src_dir = Path('.')

deduce_src = [
	x for x in src_dir.iterdir() if x.name not in ['build-scripts', 'env-vars']
]
if len(deduce_src) != 1:
	raise Exception(f'Invalid structure {deduce_src}')

base_dir = deduce_src[0]
print(f'base_dir={base_dir}')

DEFAULT_TIME = (1980, 1, 1, 0, 0, 0)

all_files: dict[str, bytes] = {}

import importlib._bootstrap_external
import importlib.util


def add_compiled(name: str, py_src: bytes):
	opt_level = 0
	pyc_name = importlib.util.cache_from_source(name, optimization=opt_level)
	code = compile(py_src, name, 'exec', dont_inherit=True, optimize=opt_level)
	source_hash = importlib.util.source_hash(py_src)

	bytecode = importlib._bootstrap_external._code_to_hash_pyc(
		code,
		source_hash,
		False,
	)

	add_file(pyc_name, bytecode, skip_pyc=False)


def add_file(name: str, contents: bytes, skip_pyc=True):
	if name in all_files:
		raise KeyError('EEXISTS')
	if skip_pyc and (name.endswith('.pyc') or name.endswith('.pyo')):
		return
	if name.endswith('/'):
		return  # skip dir
	if name == '':
		raise Exception('empty name')

	if name.endswith('.py'):
		add_compiled(name, contents)

	if name == 'runner.json':
		new_contents = (
			json.dumps(json.loads(contents), separators=(',', ':')) + '\n'
		).encode('utf-8')
		contents = new_contents

	all_files[name] = contents


for path in base_dir.glob('**/*'):
	if not path.is_file():
		continue

	add_file(str(path.relative_to(base_dir)), path.read_bytes())

assert len(all_files) != 0
assert 'runner.json' in all_files, f'files are {all_files}'

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

Path(os.environ['out']).write_bytes(tar_contents)
