import logging

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger(__name__)

from pathlib import Path
import subprocess

import lief

logging.info('Starting actual post-install script')

executor_root_dir = Path(__file__).parent.parent.parent.parent
logger.info(f'Executor root directory: {executor_root_dir}')

installation_root_dir = executor_root_dir.parent.parent

interpreter_path = installation_root_dir.joinpath('lib', 'libc.so').absolute()
logger.info(f'Interpreter path: {interpreter_path}')

if not interpreter_path.exists():
	logger.error(
		f'Interpreter path {interpreter_path} does not exist, cannot patch executables'
	)
	exit(1)


def patch_interpreter(path: Path):
	logger.info(f'Patching interpreter for {path}')
	if not path.exists():
		logger.warning(f'Path {path} does not exist, skipping interpreter patching')
		return

	binary = lief.parse(path)
	if not binary:
		logger.error(f'Failed to parse binary at {path}')
		return

	logger.info(f'Old interpreter: {binary.interpreter}')

	if Path(binary.interpreter).exists():
		logger.info(f'Interpreter {binary.interpreter} exists, skipping')
		return

	binary.interpreter = str(interpreter_path)
	binary.write(path)


patch_interpreter(executor_root_dir.joinpath('bin', 'genvm'))

patch_interpreter(installation_root_dir.joinpath('bin', 'genvm-modules'))

logger.info('checking installation')


def run_check_command(command: list[str | Path]):
	logger.info(f'Running check command: command')
	subprocess.run(command, check=True, text=True)


run_check_command([executor_root_dir.joinpath('bin', 'genvm'), '--version'])

run_check_command([installation_root_dir.joinpath('bin', 'genvm-modules'), '--version'])
