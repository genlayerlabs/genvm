from pathlib import Path
import json

MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
script_dir = Path(__file__).parent.absolute()

root_dir = script_dir
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent
MONOREPO_CONF = json.loads(root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text())

ppy_path = root_dir.joinpath(*MONOREPO_CONF['pure-py'])
import sys

sys.path.append(str(ppy_path))
