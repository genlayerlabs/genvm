import os
import json
from pathlib import Path
import sys
import typing
import enum

import numpy

import sphinx.ext.autodoc

project = 'GenLayer'
copyright = '2025, GenLayer'
author = 'GenLayer team'

extensions = [
	'sphinx.ext.autodoc',
	'sphinx.ext.viewcode',
	'sphinx.ext.todo',
	'sphinx.ext.intersphinx',
]


templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

language = 'en'

# html_theme = 'alabaster'
html_theme = 'pydata_sphinx_theme'
html_static_path = ['_static']
html_theme_options = {
	"show_nav_level": 2,
	"show_toc_level": 2,
}

todo_include_todos = True

autodoc_mock_imports = ['_genlayer_wasi', 'google', 'onnx', 'word_piece_tokenizer']

MONO_REPO_ROOT_FILE = '.genvm-monorepo-root'
script_dir = Path(__file__).parent
root_dir = script_dir
while not root_dir.joinpath(MONO_REPO_ROOT_FILE).exists():
	root_dir = root_dir.parent
MONOREPO_CONF = json.loads(root_dir.joinpath(MONO_REPO_ROOT_FILE).read_text())
sys.path.append(str(root_dir.joinpath(*MONOREPO_CONF['py-std'])))

os.environ['GENERATING_DOCS'] = 'true'

master_doc = 'index'
intersphinx_mapping = {
	'python': ('https://docs.python.org/3.12', None),
	'numpy': ('https://numpy.org/doc/stable/', None),
}

ignored_special = [
	'__dict__',
	'__abstractmethods__',
	'__annotations__',
	'__class_getitem__',
	'__init_subclass__',
	'__module__',
	'__orig_bases__',
	'__parameters__',
	'__slots__',
	'__subclasshook__',
	'__type_params__',
	'__weakref__',
	'__reversed__',
	'__protocol_attrs__',
]

autodoc_default_options: dict[str, str | bool] = {
	'inherited-members': True,
	'private-members': False,
	'special-members': True,
	'imported-members': True,
	'exclude-members': ','.join(ignored_special + ['gl']),
}

autoapi_python_class_content = 'class'
autodoc_class_signature = 'separated'
autodoc_typehints = 'both'
autodoc_typehints_description_target = 'documented_params'
autodoc_inherit_docstrings = True


def setup(app):
	def handle_bases(app, name, obj, options, bases: list):
		idx = 0
		for i in range(len(bases)):
			cur = bases[i]
			cur_name = cur if isinstance(cur, str) else cur.__name__
			if cur_name.startswith('_'):
				pass
			else:
				bases[idx] = cur
				idx += 1
		bases[idx:] = []
		if len(bases) == 0:
			bases.append(object)

	def handle_skip_member(app, what, name, obj, skip, options):
		if what == 'module' and isinstance(obj, type):
			if any(base in obj.mro() for base in [dict, tuple, bytes, enum.Enum]):
				options['special-members'] = []
				options['inherited-members'] = False
				return
		if what == 'module':
			if type(obj) is typing.NewType:
				options['special-members'] = []
				options['inherited-members'] = False
				return

	app.connect('autodoc-process-bases', handle_bases)
	app.connect('autodoc-skip-member', handle_skip_member)
