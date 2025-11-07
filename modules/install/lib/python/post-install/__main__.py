import logging
import os
import shlex

INTERACTIVE = True if os.environ.get('INTERACTIVE', 'false') == 'true' else False

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
import yaml

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


genvm_root_dir = Path(__file__).parent.parent.parent.parent
logger.info(f'Executor root directory: {genvm_root_dir}')

_interpreter_path: Path | None = None


def get_interpreter_path():
	global _interpreter_path
	if _interpreter_path is not None:
		return _interpreter_path
	interpreter_path = genvm_root_dir.joinpath('lib', 'libc.so').absolute()
	logger.info(f'Interpreter path: {interpreter_path}')

	if not interpreter_path.exists():
		logger.error(
			f'Interpreter path {interpreter_path} does not exist, cannot patch executables'
		)
		exit(1)
	_interpreter_path = interpreter_path
	return interpreter_path


def patch_executable(path: Path, *, rpath_dir: list[Path]):
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

		# Add or update RPATH entry
		if binary.has(lief.ELF.DynamicEntry.TAG.RPATH):
			# Update existing RPATH
			rpath_entry = binary.get(lief.ELF.DynamicEntry.TAG.RPATH)
			old_rpath = str(rpath_entry.value)
			rpath_entry.paths = [str(rpath) for rpath in rpath_dir]
			logger.info(f'Updated RPATH from "{old_rpath}" to: "{rpath_entry.value}"')
		else:
			# Add new RPATH entry
			rpath_entry = lief.ELF.DynamicEntryRpath([str(rpath) for rpath in rpath_dir])
			binary.add(rpath_entry)
			logger.info(f'Added new RPATH entry: "{rpath_dir}"')

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

		for rpath in rpath_dir:
			rpath_str = str(rpath)
			rpath_cmd = lief.MachO.RPathCommand.create(rpath_str)
			binary.add(rpath_cmd)
			logger.info(f'Added RPATH to Mach-O binary: "{rpath_str}"')

	else:
		logger.warning(f'Unsupported binary format for {path}: {binary.format}')
		return

	# Write the modified binary
	binary.write(str(path))
	logger.info(f'Successfully patched binary: {path}')

	if binary.format == lief.Binary.FORMATS.MACHO:
		logger.info(f'Trying to code sign Mach-O binary: {path}')
		try:
			subprocess.run(['codesign', '--force', '--sign', '-', path], check=True)
		except Exception as e:
			logger.error(
				f'Failed to code sign Mach-O binary {path}: {e}. This may be critical. You can rerun with INTERACTIVE=true environment variable to pause on this error.'
			)
			if INTERACTIVE:
				input('Waiting for user to fix the issue. Press Enter to continue...')


def run_check_command(command: list[str | Path]):
	env = os.environ.copy()
	env['LLVM_PROFILE_FILE'] = '/dev/null'
	logger.info(
		f'>> '
		+ ' '.join([shlex.quote(x if isinstance(x, str) else str(x)) for x in command])
	)
	subprocess.run(command, check=True, text=True, env=env)


modules_executable = genvm_root_dir.joinpath('bin', 'genvm-modules')
patch_executable(modules_executable, rpath_dir=[genvm_root_dir.joinpath('lib')])

run_check_command([modules_executable, '--version'])

manifest = yaml.safe_load(genvm_root_dir.joinpath('data', 'manifest.yaml').read_text())


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


def _download_single(name: str, hash: str) -> bytes:
	format_vars = {
		'name': name,
		'hash': hash,
		'hash_0_2': hash[:2],
		'hash_2_': hash[2:],
	}
	for url_template in manifest.get('runners_download_urls', []):
		url = url_template.format(**format_vars)
		try:
			logger.info(f'downloading {url}')
			with urllib.request.urlopen(url) as f:
				return f.read()
		except Exception as e:
			pass
	raise RuntimeError(f'failed to download {name}:{hash} from all sources')


for executor_version in manifest.get('executor_versions', {}).keys():
	logger.info(f'Patching executor version {executor_version}')
	executor_root_dir = genvm_root_dir.joinpath('executor', executor_version)
	executor_executable = executor_root_dir.joinpath('bin', 'genvm')
	if not executor_executable.exists():
		logger.warning(f'Executor path {executor_executable} does not exist, skipping')
		continue
	patch_executable(
		executor_executable,
		rpath_dir=[genvm_root_dir.joinpath('lib'), executor_root_dir.joinpath('lib')],
	)
	run_check_command([executor_executable, '--version'])

	logger.info(f'checking that all runners are present for {executor_version}')
	all_runners = _load_registry(executor_root_dir.joinpath('data', 'all.json'))
	runners_dir = executor_root_dir.joinpath('runners')

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

	run_check_command([executor_executable, 'precompile'])
