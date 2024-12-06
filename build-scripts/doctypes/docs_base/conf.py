import os
import json
from pathlib import Path
import sys

project = 'GenLayer'
copyright = '2024, GenLayer'
author = 'GenLayer team'

extensions = [
	'sphinx.ext.autodoc',
	'sphinx.ext.viewcode',
	'sphinx.ext.todo',
]

templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

language = 'en'

# html_theme = 'alabaster'
html_theme = 'pydata_sphinx_theme'
html_static_path = ['_static']

todo_include_todos = True

autodoc_mock_imports = ['_genlayer_wasi', 'google', 'onnx']

MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
script_dir = Path(__file__).parent
root_dir = script_dir
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent
MONOREPO_CONF = json.loads(root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text())
sys.path.append(str(root_dir.joinpath(*MONOREPO_CONF['py-std'])))

os.environ['GENERATING_DOCS'] = 'true'
