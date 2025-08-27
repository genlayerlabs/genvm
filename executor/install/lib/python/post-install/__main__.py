import logging
import os

log_level_str = os.environ.get('LOGLEVEL', 'INFO').upper()
log_levels = {
	'DEBUG': logging.DEBUG,
	'INFO': logging.INFO,
	'WARNING': logging.WARNING,
	'ERROR': logging.ERROR,
	'CRITICAL': logging.CRITICAL,
}

logging.basicConfig(level=log_levels.get(log_level_str, logging.INFO))

logger = logging.getLogger(__name__)

from pathlib import Path
import subprocess
import json

import lief

logging.info('Starting actual post-install script')

import hashlib
import urllib.request

HASH_VALID_CHARS = '0123456789abcdfghijklmnpqrsvwxyz'


def digest_to_hash_id(got_hash: bytes) -> str:
	chars = '0123456789abcdfghijklmnpqrsvwxyz'

	bytes_count = len(got_hash)
	base32_len = (bytes_count * 8 - 1) // 5 + 1

	my_hash_arr = []
	for n in range(base32_len - 1, -1, -1):
		b = n * 5
		i = b // 8
		j = b % 8
		c = (got_hash[i] >> j) | (0 if i >= bytes_count - 1 else got_hash[i + 1] << (8 - j))
		my_hash_arr.append(chars[c & 0x1F])

	return ''.join(my_hash_arr)


def runner_check_bytes(data: bytes, hash: str) -> bool:
	digest = hashlib.sha256(data).digest()
	my_hash = digest_to_hash_id(digest)
	return my_hash == hash


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


logger.info('patching interpreters')

patch_interpreter(executor_root_dir.joinpath('bin', 'genvm'))
patch_interpreter(installation_root_dir.joinpath('bin', 'genvm-modules'))

logger.info('checking that all runners are present')


def _load_registry(file: str | Path) -> dict[str, list[str]]:
	with open(file, 'r') as f:
		contents = json.load(f)

	if not isinstance(contents, dict):
		raise RuntimeError('expected dict for registry')

	ret: dict[str, list[str]] = {}

	for k, v in sorted(contents.items()):
		if isinstance(v, str):
			ret[k] = [v]
		elif isinstance(v, list):
			if not all([isinstance(x, str) for x in v]):
				raise RuntimeError(f'registry value must be str | list[str] for {k}')
			ret[k] = v

	for v in ret.values():
		v.sort()

	return ret


all_runners = _load_registry(executor_root_dir.joinpath('data', 'all.json'))
runners_dir = installation_root_dir.joinpath('runners')


def _object_gcs_path(name: str, hash: str) -> str:
	return f'genvm_runners/{name}/{hash}.tar'


def _download_single(name: str, hash: str) -> bytes:
	url = f'https://storage.googleapis.com/gh-af/{_object_gcs_path(name, hash)}'
	with urllib.request.urlopen(url) as f:
		return f.read()


for name, hashes in all_runners.items():
	for hash in hashes:
		cur_dst = runners_dir.joinpath(name, hash[:2], hash[2:] + '.tar')

		if cur_dst.exists():
			data = cur_dst.read_bytes()
			if runner_check_bytes(data, hash):
				logger.debug(f'already exists {name}:{hash}, skipping')
				continue
			logger.warning(f'exists corrupted {name}:{hash}, removing')
			cur_dst.unlink()

		logger.debug(f'not found {cur_dst}')
		data = _download_single(name, hash)
		if not runner_check_bytes(data, hash):
			raise ValueError(f'hash mismatch for {name}:{hash}')

		cur_dst.parent.mkdir(parents=True, exist_ok=True)
		cur_dst.write_bytes(data)

logger.info('checking installation')

import shlex, os


def run_check_command(command: list[str | Path]):
	env = os.environ.copy()
	env['LLVM_PROFILE_FILE'] = '/dev/null'
	logger.info(
		f'>> '
		+ ' '.join([shlex.quote(x if isinstance(x, str) else str(x)) for x in command])
	)
	subprocess.run(command, check=True, text=True, env=env)


run_check_command([executor_root_dir.joinpath('bin', 'genvm'), '--version'])
run_check_command([installation_root_dir.joinpath('bin', 'genvm-modules'), '--version'])
run_check_command([executor_root_dir.joinpath('bin', 'genvm'), 'precompile'])
