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

_interpreter_path: Path | None = None
def get_interpreter_path():
	global _interpreter_path
	if _interpreter_path is not None:
		return _interpreter_path
	interpreter_path = installation_root_dir.joinpath('lib', 'libc.so').absolute()
	logger.info(f'Interpreter path: {interpreter_path}')

	if not interpreter_path.exists():
		logger.error(
			f'Interpreter path {interpreter_path} does not exist, cannot patch executables'
		)
		exit(1)
	_interpreter_path = interpreter_path
	return interpreter_path


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

	binary.interpreter = str(get_interpreter_path())
	binary.write(path)


patch_interpreter(executor_root_dir.joinpath('bin', 'genvm'))

patch_interpreter(installation_root_dir.joinpath('bin', 'genvm-modules'))

logger.info('checking installation')

import shlex, os

def run_check_command(command: list[str | Path]):
	env = os.environ.copy()
	env['LLVM_PROFILE_FILE'] = '/dev/null'
	logger.info(f'>> ' + ' '.join([shlex.quote(x if isinstance(x, str) else str(x)) for x in command]))
	subprocess.run(command, check=True, text=True, env=env)


run_check_command([executor_root_dir.joinpath('bin', 'genvm'), '--version'])
run_check_command([installation_root_dir.joinpath('bin', 'genvm-modules'), '--version'])

run_check_command([executor_root_dir.joinpath('bin', 'genvm'), 'precompile'])
