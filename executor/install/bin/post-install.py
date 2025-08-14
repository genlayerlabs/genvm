#!/usr/bin/env python3

# NOTE: This script is a wrapper for the actual post-install script.

import logging

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger(__name__)

from pathlib import Path
import subprocess

executor_root_dir = Path(__file__).parent.parent

venv_path = executor_root_dir.joinpath('.venv')
logger.info(f'venv path: {venv_path}')

do_install = False

if not venv_path.exists():
	logger.info('venv does not exist, creating it')
	import venv

	venv.create(venv_path, with_pip=True, clear=True)

	logger.info('installing dependencies')
	do_install = True

actual_python = venv_path.joinpath('bin', 'python')

if do_install:
	logger.info('installing dependencies using')
	requirements_path = Path(__file__).parent.parent.joinpath(
		'lib', 'python', 'post-install', 'requirements.txt'
	)
	subprocess.run(
		[actual_python, '-m', 'pip', 'install', '-r', requirements_path],
		check=True,
		text=True,
	)

subprocess.run(
	[
		actual_python,
		'-B',
		executor_root_dir.joinpath('lib', 'python', 'post-install', '__main__.py'),
	],
	check=True,
	text=True,
)
