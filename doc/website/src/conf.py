import os
import json
from pathlib import Path
import sys
import typing
import enum

import sphinx.ext.autodoc

project = 'GenLayer'
copyright = '2025, GenLayer'
author = 'GenLayer team'
release = os.environ.get('DOCS_VERSION', 'main')
version = release

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
	'show_nav_level': 2,
	'show_toc_level': 2,
	'navbar_start': ['navbar-logo', 'version-switcher'],
	'switcher': {
		'json_url': f'https://{os.environ.get("DOCS_DOMAIN", "sdk.genlayer.com")}/versions.json',
		'version_match': version,
	},
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
sys.path.append(str(root_dir.joinpath(*MONOREPO_CONF['py-emb'])))

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
	'__dataclass_fields__',
	'__match_args__',
	'__dataclass_params__',
]

autodoc_default_options: dict[str, str | bool] = {
	'inherited-members': True,
	'private-members': False,
	'special-members': True,
	'exclude-members': ','.join(ignored_special + ['gl']),
}

autoapi_python_class_content = 'class'
autodoc_class_signature = 'separated'
autodoc_typehints = 'both'
autodoc_typehints_description_target = 'documented_params'
autodoc_inherit_docstrings = True
autodoc_typehints_format = 'short'
# autodoc_typehints_format = 'fully-qualified'
autodoc_preserve_defaults = True

autodoc_signature_line_length = 1


class PsuedoAll:
	def __contains__(self, _x):
		return True

	def append(self, _x):
		pass

	def __iter__(self):
		return
		yield


def setup(app):
	def handle_bases(app, name, obj, options, bases: list):
		idx = 0
		for i in range(len(bases)):
			cur = bases[i]
			cur_name = cur if isinstance(cur, str) else cur.__qualname__
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
		options['special-members'] = PsuedoAll()
		options['inherited-members'] = PsuedoAll()

	def autodoc_process_signature(
		app, what, name, obj, options, signature, return_annotation
	):
		return (signature, return_annotation)

	app.connect('autodoc-process-bases', handle_bases)
	app.connect('autodoc-skip-member', handle_skip_member)
	app.connect('autodoc-process-signature', autodoc_process_signature)


mod_names_map = {
	'genlayer.py.evm': 'genlayer.gl.evm',
	'genlayer.py.calldata': 'genlayer.gl.calldata',
	'genlayer.py.storage': 'genlayer.gl.storage',
	'genlayer.std.advanced': 'genlayer.gl.advanced',
}


def map_name(name: str) -> str:
	for k, v in mod_names_map.items():
		if name.startswith(k + '.') and len(k.split('.')) + 1 == len(name.split('.')):
			print(f'REMAP! {name}')
			return v + '.' + name[len(k) :]
	return name


from sphinx.domains.python._object import PyObject
from sphinx.domains.python import PyXRefRole

old_process_link = PyXRefRole.process_link


def new_process_link(*args, **kwargs):
	title, target = old_process_link(*args, **kwargs)

	title = map_name(title)

	return title, target


PyXRefRole.process_link = new_process_link

old_handle_signature = PyObject.handle_signature


def new_handle_signature(self, sig: str, signode) -> tuple[str, str]:
	old_modname = self.options.get('module', self.env.ref_context.get('py:module'))
	new_modname = mod_names_map.get(old_modname, old_modname)
	self.options['module'] = new_modname

	a, b = old_handle_signature(self, sig, signode)

	signode['module'] = old_modname
	self.options['module'] = old_modname

	return a, b


PyObject.handle_signature = new_handle_signature
