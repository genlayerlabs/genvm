#!/usr/bin/env python3

# NOTE: This script is a wrapper for the actual post-install script.

import logging

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger(__name__)

from pathlib import Path
import subprocess
import hashlib
import sys

genvm_root_dir = Path(__file__).parent.parent

venvs_path = genvm_root_dir.joinpath('data', 'venvs')
logger.info(f'venvs path: {venvs_path}')

requirements_path = genvm_root_dir.joinpath(
	'lib', 'python', 'post-install', 'requirements.txt'
)

create_venv_src = (
	Path(genvm_root_dir)
	.joinpath('lib', 'python', 'post-install', 'create_venv.py')
	.read_text()
)
create_venv_src_globals = {}
exec(create_venv_src, create_venv_src_globals)
create_venv = create_venv_src_globals['create_venv']
venv_path: Path = create_venv(genvm_root_dir, requirements_path)
actual_python = venv_path.joinpath('bin', 'python')

subprocess.run(
	[
		actual_python,
		'-B',
		genvm_root_dir.joinpath('lib', 'python', 'post-install', '__main__.py'),
	],
	check=True,
	text=True,
)
