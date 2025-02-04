from pathlib import Path

test_dir = Path(__file__).parent

root_dir = test_dir
while not root_dir.joinpath('.genvm-monorepo-root').exists():
	root_dir = root_dir.parent

config_path = root_dir.joinpath('build', 'config.json')
if config_path.exists():
	import json

	with open(config_path, 'rt') as f:
		dat = json.load(f)

	original_text = test_dir.joinpath('dup-dependency.py').read_text()

	import re

	new_text = re.sub(
		'"softfloat:[^"]*"', f'"softfloat:{dat['runners']['softfloat']}"', original_text
	)
	test_dir.joinpath('dup-dependency.py').write_text(new_text)
