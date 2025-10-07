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

rpath_dir = installation_root_dir.joinpath('lib')

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


def patch_executable(path: Path):
	logger.info(f'Patching executable for {path}')
	if not path.exists():
		logger.warning(f'Path {path} does not exist, skipping patching')
		return

	binary = lief.parse(path)
	if not binary:
		logger.error(f'Failed to parse binary at {path}')
		return

	# Log basic binary information
	logger.info(f'Binary format: {binary.format}')

	# Handle ELF binaries
	if binary.format == lief.Binary.FORMATS.ELF:
		logger.info(f'Processing ELF binary: {path}')

		# Log current interpreter
		logger.info(f'Current interpreter: {binary.interpreter}')

		# Log current needed libraries
		needed_libs = [
			lib if isinstance(lib, str) else lib.name for lib in binary.libraries
		]
		logger.info(f'Current needed libraries: {needed_libs}')

		# Log current RPATH/RUNPATH
		rpath_entries = []
		if binary.has(lief.ELF.DynamicEntry.TAG.RPATH):
			rpath_entry = binary.get(lief.ELF.DynamicEntry.TAG.RPATH)
			rpath_entries.append(f'RPATH: {rpath_entry.value}')
		if binary.has(lief.ELF.DynamicEntry.TAG.RUNPATH):
			runpath_entry = binary.get(lief.ELF.DynamicEntry.TAG.RUNPATH)
			rpath_entries.append(f'RUNPATH: {runpath_entry.value}')
		logger.info(
			f'Current RPATH/RUNPATH entries: {rpath_entries if rpath_entries else "None"}'
		)

		# Patch interpreter only for ELF
		if Path(binary.interpreter).exists():
			logger.info(
				f'Interpreter {binary.interpreter} exists, skipping interpreter patching'
			)
		else:
			old_interpreter = binary.interpreter
			new_interpreter = str(get_interpreter_path())
			binary.interpreter = new_interpreter
			logger.info(f'Updated interpreter from {old_interpreter} to: {new_interpreter}')

		# Update RPATH for ELF
		logger.info(f'Updating RPATH for ELF binary')
		rpath_str = str(rpath_dir)

		# Add or update RPATH entry
		if binary.has(lief.ELF.DynamicEntry.TAG.RPATH):
			# Update existing RPATH
			rpath_entry = binary.get(lief.ELF.DynamicEntry.TAG.RPATH)
			old_rpath = rpath_entry.value
			rpath_entry.value = rpath_str
			logger.info(f'Updated RPATH from "{old_rpath}" to: "{rpath_str}"')
		else:
			# Add new RPATH entry
			rpath_entry = lief.ELF.DynamicEntryRpath([rpath_str])
			binary.add(rpath_entry)
			logger.info(f'Added new RPATH entry: "{rpath_str}"')

	# Handle Mach-O binaries
	elif binary.format == lief.Binary.FORMATS.MACHO:
		logger.info(f'Processing Mach-O binary: {path}')

		# Log current RPATH commands
		existing_rpaths = []
		for cmd in binary.commands:
			if cmd.command == lief.MachO.LoadCommand.TYPE.RPATH:
				existing_rpaths.append(cmd.path)
		logger.info(
			f'Current RPATH entries: {existing_rpaths if existing_rpaths else "None"}'
		)

		# Log current needed libraries
		needed_libs = []
		for cmd in binary.commands:
			if cmd.command in [
				lief.MachO.LoadCommand.TYPE.LOAD_DYLIB,
				lief.MachO.LoadCommand.TYPE.LOAD_WEAK_DYLIB,
			]:
				needed_libs.append(cmd.name)
		logger.info(f'Current needed libraries: {needed_libs}')

		# Replace specific library references
		for cmd in binary.commands:
			if cmd.command in [
				lief.MachO.LoadCommand.TYPE.LOAD_DYLIB,
				lief.MachO.LoadCommand.TYPE.LOAD_WEAK_DYLIB,
			]:
				if cmd.name == '/usr/local/lib/libiconv.2.dylib':
					old_name = cmd.name
					cmd.name = '@rpath/libiconv.dylib'
					logger.info(f'Replaced library reference: "{old_name}" -> "{cmd.name}"')
				elif '/' not in cmd.name:
					old_name = cmd.name
					cmd.name = '@rpath/' + cmd.name
					logger.info(f'Replaced library reference: "{old_name}" -> "{cmd.name}"')

		# Set RPATH for Mach-O
		rpath_str = str(rpath_dir)

		# Add RPATH load command
		rpath_cmd = lief.MachO.RPathCommand.create(rpath_str)
		binary.add(rpath_cmd)
		logger.info(f'Added RPATH to Mach-O binary: "{rpath_str}"')

	else:
		logger.warning(f'Unsupported binary format for {path}: {binary.format}')
		return

	# Write the modified binary
	binary.write(str(path))
	logger.info(f'Successfully patched binary: {path}')


logger.info('patching interpreters')

patch_executable(executor_root_dir.joinpath('bin', 'genvm'))
patch_executable(installation_root_dir.joinpath('bin', 'genvm-modules'))

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
