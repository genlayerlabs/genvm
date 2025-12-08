#!/usr/bin/env python3

# NOTE: This script is a wrapper for the actual post-install script.

import logging

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger(__name__)

from pathlib import Path
import subprocess
import sys
import argparse


def str_to_bool(value):
	if value.lower() in ('yes', 'true', 't', 'y', '1'):
		return True
	elif value.lower() in ('no', 'false', 'f', 'n', '0'):
		return False
	else:
		raise argparse.ArgumentTypeError('Boolean value expected.')


parser = argparse.ArgumentParser(add_help=False)
parser.add_argument(
	'--create-venv',
	type=str_to_bool,
	default=True,
	help='Whether to create a virtual environment (true/false)',
)

args, rest_args = parser.parse_known_args()

if '-h' in rest_args or '--help' in rest_args:
	parser.print_help()

genvm_root_dir = Path(__file__).parent.parent

if args.create_venv:
	venvs_path = genvm_root_dir.joinpath('data', 'venvs')
	logger.info(f'venvs path: {venvs_path}')

	requirements_path = genvm_root_dir.joinpath(
		'lib', 'python', 'post-install', 'requirements.txt'
	)

	create_venv_path = Path(genvm_root_dir).joinpath(
		'lib', 'python', 'post-install', 'create_venv.py'
	)

	create_venv_src = create_venv_path.read_text()
	create_venv_src_globals = {}
	create_venv_code = compile(create_venv_src, str(create_venv_path), 'exec')
	exec(create_venv_code, create_venv_src_globals)
	create_venv = create_venv_src_globals['create_venv']
	venv_path: Path = create_venv(genvm_root_dir, requirements_path)
	actual_python = venv_path.joinpath('bin', 'python')
else:
	actual_python = Path(sys.executable)


subprocess.run(
	[
		actual_python,
		'-B',
		genvm_root_dir.joinpath('lib', 'python', 'post-install', '__main__.py'),
	]
	+ rest_args,
	check=True,
	text=True,
)
