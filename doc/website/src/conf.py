import datetime
import os
import inspect
import json
from pathlib import Path
import sys
from types import ModuleType
import typing
import enum

import sphinx.ext.autodoc
import sphinx.util.inspect as _sphinx_inspect

# Patch stringify_annotation to replace Annotated[int, StaticIntMeta(...)] with u8..u256/i8..i256
_orig_stringify_annotation = _sphinx_inspect.stringify_annotation

_annotated_type_aliases: dict[typing.Any, str] = {}


project = 'GenVM SDK'

copyright = f'{datetime.date.today().year}, GenLayer Labs'
author = 'GenLayer Labs'
release = os.environ.get('DOCS_VERSION', 'main')
version = release

extensions = [
	'sphinx.ext.autodoc',
	'sphinx.ext.viewcode',
	'sphinx.ext.todo',
	'sphinx.ext.intersphinx',
	'sphinxcontrib.mermaid',
	'sphinxcontrib.openapi',
	'myst_parser',
]

templates_path = ['_templates']
exclude_patterns = [
	'_build',
	'Thumbs.db',
	'.DS_Store',
	'*_generated.rst',
	'api',
	'impl-spec/appendix/runners-versions.rst',
]

language = 'en'

mermaid_version = '11.6.0'
mermaid_output_format = 'svg'
mermaid_params = ['--theme', 'dark', '--backgroundColor', 'transparent']

# html_theme = 'alabaster'
html_theme = 'pydata_sphinx_theme'
html_static_path = ['_static']

_docs_domain = os.environ.get('DOCS_DOMAIN', 'sdk.genlayer.com')

# Fetch remote versions and merge with local build, sorted by semver
import re as _re, urllib.request

_switcher_url = '/_static/versions.json'
try:
	_req = urllib.request.Request(
		f'https://{_docs_domain}/versions.json', headers={'User-Agent': 'sphinx-build'}
	)
	with urllib.request.urlopen(_req, timeout=5) as _resp:
		_remote_versions = json.loads(_resp.read())
except Exception:
	_remote_versions = []

# Add local build entry
_local_entry = {
	'name': f'{version} (local)',
	'version': version,
	'url': '/',
	'preferred': True,
}
_versions = [v for v in _remote_versions if v.get('version') != version] + [
	_local_entry
]


# Sort by semver descending, non-semver (like "main") at end
def _semver_sort_key(entry):
	m = _re.match(r'^v?(\d+)\.(\d+)\.(\d+)', entry.get('version', ''))
	if m:
		return (0, -int(m.group(1)), -int(m.group(2)), -int(m.group(3)))
	return (1, entry.get('version', ''))


_versions.sort(key=_semver_sort_key)

Path(__file__).parent.joinpath('_static', 'versions.json').write_text(
	json.dumps(_versions, indent=2)
)

html_theme_options = {
	'logo': {
		'image_light': '_static/logo-light.svg',
		'image_dark': '_static/logo-dark.svg',
	},
	'show_nav_level': 2,
	'show_toc_level': 2,
	'navbar_start': ['navbar-logo', 'version-switcher'],
	'navbar_end': ['theme-switcher', 'navbar-icon-links'],
	'icon_links': [
		{
			'name': 'Full docs for LLMs',
			'url': '/_static/llms.txt',
			'icon': 'fa-solid fa-robot',
			'type': 'fontawesome',
		},
		{
			'name': 'GitHub',
			'url': 'https://github.com/genlayerlabs/genvm',
			'icon': 'fa-brands fa-github',
			'type': 'fontawesome',
		},
		{
			'name': 'Discord',
			'url': 'https://discord.gg/8Jm4v89VAu',
			'icon': 'fa-brands fa-discord',
			'type': 'fontawesome',
		},
		{
			'name': 'Telegram',
			'url': 'https://t.me/genlayer',
			'icon': 'fa-brands fa-telegram',
			'type': 'fontawesome',
		},
		{
			'name': 'X (Twitter)',
			'url': 'https://x.com/GenLayer',
			'icon': 'fa-brands fa-x-twitter',
			'type': 'fontawesome',
		},
	],
	'primary_sidebar_end': ['version-switcher'],
	'footer_start': ['copyright'],
	'footer_end': [],
	'switcher': {
		'json_url': _switcher_url,
		'version_match': version,
	},
}

html_show_sourcelink = False
html_css_files = ['custom.css']
html_js_files = ['favicon-swap.js']
html_favicon = '_static/favicon.png'

todo_include_todos = True

autodoc_mock_imports = ['_genlayer_wasi', 'google', 'onnx', 'word_piece_tokenizer']

fake_genlayer_wasi = ModuleType(
	'_genlayer_wasi',
)
fake_genlayer_wasi.__dict__['FAKE_VM'] = True
sys.modules['_genlayer_wasi'] = fake_genlayer_wasi

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
		import types

		if 'what' == 'class' and isinstance(obj, types.MethodType):
			if obj.__self__.__class__ is typing.NewType:
				return True
		if what == 'module' and isinstance(obj, type):
			if any(base in obj.mro() for base in [dict, tuple, bytes, enum.Enum]):
				options['special-members'] = []
				options['inherited-members'] = False
				return
		if what == 'module':
			if isinstance(obj, typing.NewType):
				options['special-members'] = []
				options['inherited-members'] = False
				return
		# options['special-members'] = PsuedoAll()
		# options['inherited-members'] = PsuedoAll()

	# Build u8..u256, i8..i256 display aliases for Annotated[int, StaticIntMeta(...)]
	_type_display_aliases = {}
	for _prefix, _signed in [('u', False), ('i', True)]:
		for _sz in range(1, 33):
			_bits = _sz * 8
			_type_display_aliases[
				f'typing.Annotated[int, StaticIntMeta(size={_sz}, signed={_signed})]'
			] = f'genlayer.types.{_prefix}{_bits}'
			_type_display_aliases[
				f'Annotated[int, StaticIntMeta(size={_sz}, signed={_signed})]'
			] = f'genlayer.types.{_prefix}{_bits}'

	def autodoc_process_signature(
		app, what, name, obj, options, signature, return_annotation
	):
		for old, new in _type_display_aliases.items():
			if signature:
				signature = signature.replace(old, new)
			if return_annotation:
				return_annotation = return_annotation.replace(old, new)
		return (signature, return_annotation)

	app.connect('autodoc-process-bases', handle_bases)
	app.connect('autodoc-skip-member', handle_skip_member)
	app.connect('autodoc-process-signature', autodoc_process_signature)
