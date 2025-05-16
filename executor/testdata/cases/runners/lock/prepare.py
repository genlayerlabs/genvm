from pathlib import Path

test_dir = Path(__file__).parent

root_dir = test_dir
while not root_dir.joinpath('.genvm-monorepo-root').exists():
	root_dir = root_dir.parent

config_path = root_dir.joinpath(
	*'build/out/share/lib/genvm/runners/latest.json'.split('/')
)

if config_path.exists():
	import json

	with open(config_path, 'rt') as f:
		dat = json.load(f)
	with open(test_dir.joinpath('runner.json'), 'rt') as f:
		cur_contract = json.load(f)
	cur_contract['Seq'][2]['Depends'] = f"softfloat:{dat['softfloat']}"
	cur_contract['Seq'][4]['With']['runner'] = f"cpython:{dat['cpython']}"
	with open(test_dir.joinpath('runner.json'), 'wt') as f:
		json.dump(cur_contract, f, separators=(',', ': '), indent=2)
		f.write('\n')

import zipfile

with zipfile.ZipFile(test_dir.joinpath('contract.zip'), 'w') as f:
	for name in ['contract.py', 'runner.json', 'new_json.py']:
		f.write(test_dir.joinpath(name), name)
