from pathlib import Path
import sys
import hashlib
import logging
import subprocess

logger = logging.getLogger(__name__)


def create_venv(genvm_root_path, requirements_path: Path) -> Path:
	venvs_path = genvm_root_path.joinpath('data', 'venvs')

	requirements = '\n'.join(
		sorted(
			line.strip()
			for line in requirements_path.read_text().splitlines()
			if line.strip() and not line.startswith('#')
		)
	)

	requirements = f'# {sys.version}\n' + requirements + '\n'

	requirements_hash = hashlib.sha3_256(requirements.encode('utf-8')).hexdigest()[:16]
	venv_path = venvs_path.joinpath(f'{requirements_hash}')

	if venv_path.exists():
		return venv_path

	logger.info(f'venv {venv_path} does not exist, creating it')
	import venv

	venv.create(venv_path, with_pip=True, clear=True)

	actual_python = venv_path.joinpath('bin', 'python')

	logger.info('installing dependencies')

	subprocess.run(
		[actual_python, '-m', 'pip', 'install', '-r', requirements_path],
		check=True,
		text=True,
	)

	return venv_path
