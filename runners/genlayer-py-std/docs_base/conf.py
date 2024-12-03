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

html_theme = 'alabaster'
html_static_path = ['_static']

todo_include_todos = True

autodoc_mock_imports = ['_genlayer_wasi', 'google', 'onnx']

import os

os.environ['GENERATING_DOCS'] = 'true'
