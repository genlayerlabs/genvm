import logging
import os
import shlex
import argparse
import platform
import tarfile
import io
import traceback
import hashlib

target_os = platform.system().lower()
if target_os == 'darwin':
	target_os = 'macos'
if target_os not in ('linux', 'macos'):
	target_os = 'linux'  # default to linux


target_arch = platform.machine()
target_arch = {
	'x86_64': 'amd64',
	'aarch64': 'arm64',
	'arm64': 'arm64',
	'amd64': 'amd64',
}.get(target_arch, 'amd64')


def str_to_bool(value):
	if value.lower() in ('yes', 'true', 't', 'y', '1'):
		return True
	elif value.lower() in ('no', 'false', 'f', 'n', '0'):
		return False
	else:
		raise argparse.ArgumentTypeError('Boolean value expected.')


def str_to_bool_or_none(value):
	if value is None or value.lower() in ('none', 'default', 'null', 'nil', ''):
		return None
	return str_to_bool(value)


parser = argparse.ArgumentParser()
parser.add_argument(
	'--os',
	type=str,
	default=target_os,
	help='Target operating system (linux/macos)',
)
parser.add_argument(
	'--arch',
	type=str,
	default=target_arch,
	help='Target architecture (amd64/arm64)',
)
parser.add_argument(
	'--error-on-missing-executor',
	type=str_to_bool,
	default=True,
	help='Whether to error on missing executor',
)
parser.add_argument(
	'--log-level',
	type=str,
	default='INFO',
	help='Logging level',
	choices=[
		'DEBUG',
		'INFO',
		'WARNING',
		'ERROR',
		'CRITICAL',
	],
)

step_names = [
	'executor-download',
	'runners-download',
	'bin-patch',
	'bin-check',
	'precompile',
]

parser.add_argument(
	'--default-steps',
	type=str_to_bool,
	default=True,
	help='Default value for steps when not explicitly set',
)
parser.add_argument(
	'--default-download',
	type=str_to_bool_or_none,
	default=None,
	help='Default value for download steps (default: use --default-steps)',
)
parser.add_argument(
	'--executor-download',
	type=str_to_bool_or_none,
	default=None,
	help='Enable/disable executor download step (default: use --default-download)',
)
parser.add_argument(
	'--runners-download',
	type=str_to_bool_or_none,
	default=None,
	help='Enable/disable runners download step (default: use --default-download)',
)
parser.add_argument(
	'--bin-patch',
	type=str_to_bool_or_none,
	default=None,
	help='Enable/disable bin patch step (default: use --default-steps)',
)
parser.add_argument(
	'--bin-check',
	type=str_to_bool_or_none,
	default=None,
	help='Enable/disable bin check step (default: use --default-steps)',
)
parser.add_argument(
	'--precompile',
	type=str_to_bool_or_none,
	default=None,
	help='Enable/disable precompile step (default: use --default-steps)',
)

args = parser.parse_args()

# Apply default to None values
if args.default_download is None:
	args.default_download = args.default_steps

if args.executor_download is None:
	args.executor_download = args.default_download
if args.runners_download is None:
	args.runners_download = args.default_download
if args.bin_patch is None:
	args.bin_patch = args.default_steps
if args.bin_check is None:
	args.bin_check = args.default_steps
if args.precompile is None:
	args.precompile = args.default_steps

log_level_str = args.log_level
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

logging.info('Starting actual post-install script')

# Interpreter patching rewrites the ELF interpreter (dynamic loader) to the
# bundled one, whose path is only known at install time. It applies to ELF
# binaries (Linux) only; on other targets the loader is fixed and rpath/needed
# entries are already set correctly at build time, so lief is not needed.
patch_interpreter = args.bin_patch and args.os == 'linux'

if patch_interpreter:
	import lief

	lief.logging.set_level(lief.logging.LEVEL.ERROR)

# gvm32 (Crockford's Base32, big-endian, no padding) — the encoding the executor
# uses for runner hashes / on-disk paths. MUST match support/runner-script.py,
# executor/crates/sdk-rs/src/gvm32.rs and runners/genlayer-py-std/.../gvm32.py.
# NOT Nix base32 (which uses a different alphabet AND little-endian bit order).
HASH_VALID_CHARS = '0123456789abcdefghjkmnpqrstvwxyz'


def digest_to_hash_id(got_hash: bytes) -> str:
	out = []
	value = 0
	bits = 0
	for byte in got_hash:
		value = (value << 8) | byte
		bits += 8
		while bits >= 5:
			bits -= 5
			out.append(HASH_VALID_CHARS[(value >> bits) & 0x1F])
	if bits > 0:
		out.append(HASH_VALID_CHARS[(value << (5 - bits)) & 0x1F])
	return ''.join(out)


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


def patch_executable(path: Path):
	logger.info(f'Patching interpreter for {path}')
	if not path.exists():
		logger.warning(f'Path {path} does not exist, skipping patching')
		return

	binary = lief.parse(path)
	if not binary:
		logger.error(f'Failed to parse binary at {path}')
		return

	# rpath/needed entries are set correctly at build time, so the only install
	# time fixup left is the ELF interpreter (dynamic loader) path.
	if binary.format != lief.Binary.FORMATS.ELF:
		logger.info(f'{path} is not ELF ({binary.format}), nothing to patch')
		return

	logger.info(f'Current interpreter: {binary.interpreter}')
	if Path(binary.interpreter).exists():
		logger.info(
			f'Interpreter {binary.interpreter} exists, skipping interpreter patching'
		)
		return

	old_interpreter = binary.interpreter
	binary.interpreter = str(get_interpreter_path())
	logger.info(f'Updated interpreter from {old_interpreter} to: {binary.interpreter}')

	binary.write(str(path))
	logger.info(f'Successfully patched interpreter: {path}')


def run_check_command(command: list[str | Path]):
	env = os.environ.copy()
	env['LLVM_PROFILE_FILE'] = '/dev/null'
	logger.info(
		f'>> '
		+ ' '.join([shlex.quote(x if isinstance(x, str) else str(x)) for x in command])
	)
	subprocess.run(command, check=True, text=True, env=env)


modules_executable = genvm_root_dir.joinpath('bin', 'genvm-modules')
if patch_interpreter:
	patch_executable(modules_executable)

if args.bin_check:
	run_check_command([modules_executable, '--version'])


def load_micro_yaml():
	src = Path(__file__).parent.joinpath('micro_yaml.py')
	globs = {}
	code = compile(src.read_text(), str(src), 'exec')
	exec(code, globs)
	return globs['loads']


manifest = load_micro_yaml()(
	genvm_root_dir.joinpath('data', 'manifest.yaml').read_text()
)


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


def _download_url(url: str) -> bytes:
	import urllib.request

	logger.info(f'downloading {url}')
	for attempt in range(5):
		try:
			with urllib.request.urlopen(url) as f:
				return f.read()
		except Exception as e:
			trace = traceback.format_exception(e)
			logger.warning(f'Attempt {attempt + 1} failed for {url}: {e}' + ''.join(trace))
	raise RuntimeError(f'failed to download {url} after multiple attempts')


def _download_template(descr: str, templates: list[str], vars: dict[str, str]) -> bytes:
	for url_template in templates:
		url = url_template.format(**vars)
		try:
			logger.info(f'downloading {url}')
			return _download_url(url)
		except Exception as e:
			pass
	raise RuntimeError(f'failed to download {descr}{vars} from all sources')


def download_runners_from_json(file: str | Path):
	file = Path(file)
	if not file.exists():
		if args.error_on_missing_executor:
			logger.error(f'Executor path {file} does not exist')
			raise RuntimeError(f'Executor path {file} does not exist')
		else:
			logger.warning(f'Executor path {file} does not exist, skipping')
			return
	logger.info(f'checking that all runners are present for {file}')
	all_runners = _load_registry(file)
	runners_dir = genvm_root_dir.joinpath('runners')

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
			data = _download_template(
				f'runner {name}',
				manifest.get('runners_download_urls', []),
				{
					'name': name,
					'hash': hash,
					'hash_0_2': hash[:2],
					'hash_2_': hash[2:],
				},
			)
			if not runner_check_bytes(data, hash):
				raise ValueError(f'hash mismatch for {name}:{hash}')

			cur_dst.parent.mkdir(parents=True, exist_ok=True)
			cur_dst.write_bytes(data)


all_executor_versions = list(manifest.get('executor_versions', {}).keys())
all_executor_versions.sort()


def parse_executor_version(executor_version: str) -> tuple[int, int, int]:
	executor_version = executor_version.removeprefix('v')
	major_str, minor_str, patch_str = executor_version.split('.', 2)
	patch_str = patch_str.split('-', 1)[0]
	return (int(major_str), int(minor_str), int(patch_str))


all_executor_version_tuples = set()
for v in all_executor_versions:
	if v != 'vTEST':
		all_executor_version_tuples.add(parse_executor_version(v))


def process_executor_version(executor_version: str):
	logger.info(f'Examining executor version {executor_version}')

	if executor_version != 'vTEST':
		major, minor, patch = parse_executor_version(executor_version)
		if (major, minor, patch + 1) in all_executor_version_tuples:
			logger.info(
				f'Skipping executor version {executor_version} because a newer version {next_version} exists'
			)
			return

	executor_root_dir = genvm_root_dir.joinpath('executor', executor_version)
	executor_executable = executor_root_dir.joinpath('bin', 'genvm')

	if args.executor_download or patch_interpreter or args.bin_check:
		if not executor_executable.exists() and args.executor_download:
			import lzma

			tar_xz_data = _download_template(
				'executor',
				manifest.get('executor_download_urls', []),
				{
					'executor_version': executor_version,
					'target_os': args.os,
					'target_arch': args.arch,
				},
			)
			tar_data = lzma.decompress(tar_xz_data)

			# Integrity check before extraction. The expected hash, when present
			# in the manifest, is taken over the decompressed `.tar` (matching the
			# runner convention in runner_check_bytes), not the `.tar.xz`. No
			# executor hash is published yet, so this currently warns instead of
			# failing; add a `hash` field to the manifest's executor_versions entry
			# to enforce it.
			expected_hash = (
				manifest.get('executor_versions', {}).get(executor_version, {}).get('hash')
			)
			if expected_hash is not None:
				if not runner_check_bytes(tar_data, expected_hash):
					raise ValueError(f'hash mismatch for executor {executor_version}')
			elif executor_version == 'vTEST':
				logger.warning(
					f'no integrity hash for executor {executor_version}; skipping verification'
				)
			else:
				raise ValueError(
					f'no integrity hash for executor {executor_version}; refusing to extract'
				)

			# filter='data' rejects absolute / `..` paths and unsafe link and device
			# members, preventing writes outside genvm_root_dir (tar path-traversal /
			# link-abuse).
			with tarfile.TarFile.open(fileobj=io.BytesIO(tar_data)) as tar:
				tar.extractall(path=genvm_root_dir, filter='data')
		if not executor_executable.exists():
			if args.error_on_missing_executor:
				logger.error(f'Executor path {executor_executable} does not exist')
				raise RuntimeError(f'Executor path {executor_executable} does not exist')
			else:
				logger.warning(f'Executor path {executor_executable} does not exist, skipping')
				return
	if patch_interpreter:
		patch_executable(executor_executable)
	if args.bin_check:
		run_check_command([executor_executable, '--version'])

	if args.runners_download:
		download_runners_from_json(executor_root_dir.joinpath('data', 'all.json'))

	if args.precompile:
		logger.info(f'Precompiling executor {executor_version}')
		run_check_command([executor_executable, 'precompile'])


for executor_version in all_executor_versions:
	process_executor_version(executor_version)

if genvm_root_dir.joinpath('executor', 'vTEST').exists():
	process_executor_version('vTEST')
