from pathlib import Path

test_dir = Path(__file__).parent

import zipfile

with zipfile.ZipFile(test_dir.joinpath('contract.zip'), 'w') as f:
	for name in ['__init__.py', 'lib.py']:
		f.write(test_dir.joinpath(name), 'contract/' + name)
	f.write(test_dir.joinpath('runner.json'), 'runner.json')
