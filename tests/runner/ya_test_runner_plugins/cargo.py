import json
import os
import shutil
from pathlib import Path
import subprocess
import sys

import ya_test_runner
from ya_test_runner.test import CONST_PASSED

local_ctx = ya_test_runner.stage.configuration.current_context()

build_info = json.loads(
	local_ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
)

BUILD_DIR = Path(build_info['build_dir'])
TARGET_DIR = Path(build_info['rust_target_dir'])
COVERAGE_DIR = Path(build_info['coverage_dir'])

# Add --coverage flag to run parser
local_ctx.run_parser.add_argument(
	'--coverage',
	action='store_true',
	default=False,
	help='Enable coverage collection for Rust tests',
)

# Track profile objects for coverage (populated during collection)
_profile_objects: list[Path] = []


def _is_coverage_enabled() -> bool:
	"""Check if coverage is enabled (after args are parsed)."""
	return '--coverage' in sys.argv


def _get_default_env() -> dict[str, str]:
	"""Get default environment for cargo commands."""
	env = {
		k: v
		for k, v in os.environ.items()
		if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
	}

	if _is_coverage_enabled():
		# Enable coverage instrumentation
		env['RUSTFLAGS'] = '-C instrument-coverage'
		env['LLVM_PROFILE_FILE'] = str(COVERAGE_DIR / 'cov-%p-%16m.profraw')
	else:
		env['LLVM_PROFILE_FILE'] = '/dev/null'

	env['AFL_FUZZER_LOOPCOUNT'] = '20'  # without it no coverage will be written!
	env['AFL_NO_CFG_FUZZING'] = '1'
	env['AFL_BENCH_UNTIL_CRASH'] = '1'

	return env


def _find_llvm_tool(tool_name: str) -> str:
	"""Find LLVM tool, preferring the one from rustc's toolchain."""
	# Try to find from rustc's toolchain first
	try:
		result = subprocess.run(
			['rustc', '--print', 'target-libdir'],
			capture_output=True,
			text=True,
			check=True,
		)
		llvm_bin = Path(result.stdout.strip()).parent / 'bin'
		tool_path = llvm_bin / tool_name
		if tool_path.exists():
			return str(tool_path)
	except (subprocess.CalledProcessError, FileNotFoundError):
		pass

	# Fall back to system PATH
	system_tool = shutil.which(tool_name)
	if system_tool:
		return system_tool

	raise RuntimeError(f'{tool_name} not found')


def _coverage_post_run(
	shared: ya_test_runner.SharedContext,
	execution_env: ya_test_runner.stage.execution.Env,
) -> None:
	"""Post-run step for coverage collection."""
	shared.logger.info('Collecting coverage data', coverage_dir=str(COVERAGE_DIR))

	# Find all .profraw files
	profraw_files = list(COVERAGE_DIR.glob('*.profraw'))
	if not profraw_files:
		shared.logger.warning('No .profraw files found', coverage_dir=str(COVERAGE_DIR))
		return

	shared.logger.info('Found profraw files', count=len(profraw_files))

	# Write file list
	files_list_path = COVERAGE_DIR / 'files-list'
	files_list_path.write_text('\n'.join(str(f) for f in profraw_files))

	# Find LLVM tools
	try:
		llvm_profdata = _find_llvm_tool('llvm-profdata')
		llvm_cov = _find_llvm_tool('llvm-cov')
	except RuntimeError as e:
		shared.logger.error('LLVM tools not found', error=str(e))
		return

	# Merge profraw files
	merged_profdata = COVERAGE_DIR / 'merged.profdata'
	shared.logger.info('Merging profile data')

	merge_cmd = [
		llvm_profdata,
		'merge',
		'-sparse',
		'-f',
		str(files_list_path),
		'-o',
		str(merged_profdata),
	]

	result = subprocess.run(merge_cmd, capture_output=True, text=True)
	if result.returncode != 0:
		shared.logger.error(
			'Failed to merge profile data',
			returncode=result.returncode,
			stderr=result.stderr,
		)
		return

	# Collect existing profile objects, expanding directories
	existing_objects: list[Path] = []
	for obj in _profile_objects:
		if obj.is_dir():
			# For directories (like deps/), find executable files
			for child in obj.iterdir():
				if child.is_file() and not child.suffix and child.stat().st_mode & 0o111:
					existing_objects.append(child)
		elif obj.exists():
			existing_objects.append(obj)

	if not existing_objects:
		shared.logger.warning('No profile objects found')
		return

	shared.logger.info('Found profile objects', count=len(existing_objects))

	# Generate coverage report
	report_path = COVERAGE_DIR / 'report.txt'
	shared.logger.info('Generating coverage report')

	cov_cmd = [
		llvm_cov,
		'report',
		'-format=text',
		f'-instr-profile={merged_profdata}',
		'--ignore-filename-regex=(^|/)(\\.cargo|\\.rustup|third-party)/|cranelift|target-lexicon',
	]

	for obj in existing_objects:
		cov_cmd.extend(['--object', str(obj)])

	result = subprocess.run(cov_cmd, capture_output=True, text=True)
	if result.returncode != 0:
		shared.logger.error(
			'Failed to generate coverage report',
			returncode=result.returncode,
			stderr=result.stderr,
		)
		return

	# Write and display report
	report_path.write_text(result.stdout)
	shared.printer.put('coverage report', path=str(report_path))

	# Print the report to stdout
	print(result.stdout)


# Register coverage post-run step if coverage is enabled
if _is_coverage_enabled():
	local_ctx.add_post_run_step(_coverage_post_run)
	# Add genvm binaries to profile objects
	_profile_objects.append(BUILD_DIR / 'out' / 'bin' / 'genvm')
	_profile_objects.append(BUILD_DIR / 'out' / 'bin' / 'genvm-modules')


def cargo_test(
	ctx: ya_test_runner.stage.collection.Context,
	desc: ya_test_runner.test.Description,
	*,
	rust_root_dir: Path,
):
	desc = desc.with_tags(['rust', 'unit'])._replace(
		console_pool=True,
	)

	test_env = _get_default_env()

	extra_flags = []

	extra_config = rust_root_dir.joinpath('.ya-test-config.json')
	if extra_config.exists():
		extra_conf = json.loads(extra_config.read_text())
		extra_flags.extend(extra_conf.get('cargo_test_flags', []))
		for name in extra_conf.get('keep_env', []):
			if name in os.environ:
				test_env[name] = os.environ[name]

	# Track deps directory for coverage
	if _is_coverage_enabled():
		deps_dir = TARGET_DIR / 'debug' / 'deps'
		if deps_dir not in _profile_objects:
			_profile_objects.append(deps_dir)

	case = ya_test_runner.test.SimpleCommandCase(
		description=desc,
		command=[
			'cargo',
			'test',
			'--color=always',
			'--target-dir',
			TARGET_DIR,
			'--tests',
		]
		+ extra_flags,
		cwd=rust_root_dir,
		env=test_env,
		mode=ya_test_runner.exec.command.RunMode.INTERACTIVE,
	)

	ctx.add_case(case)


def cargo_fuzz(
	ctx: ya_test_runner.stage.collection.Context,
	desc: ya_test_runner.test.Description,
	*,
	rust_root_dir: Path,
	name: str,
):
	desc = desc.with_tags(['rust', 'fuzz'])._replace(
		console_pool=True,
	)

	test_env = _get_default_env()

	extra_flags = []

	extra_config = rust_root_dir.joinpath('.ya-test-config.json')
	if extra_config.exists():
		extra_conf = json.loads(extra_config.read_text())
		extra_flags.extend(extra_conf.get('cargo_test_flags', []))

	# Track fuzz binary for coverage
	if _is_coverage_enabled():
		fuzz_binary = TARGET_DIR / 'debug' / 'examples' / f'fuzz-{name}'
		if fuzz_binary not in _profile_objects:
			_profile_objects.append(fuzz_binary)

	steps: list[ya_test_runner.exec.step.Step] = []
	steps.extend(
		[
			ya_test_runner.exec.step.SetCwd(path=rust_root_dir),
		]
		+ [ya_test_runner.exec.step.SetEnv(key=k, value=v) for k, v in test_env.items()]
		+ [
			ya_test_runner.exec.step.Run(
				args=[
					'cargo',
					'afl',
					'build',
					'--target-dir',
					TARGET_DIR,
					'--example',
					f'fuzz-{name}',
					'--color=always',
				]
				+ extra_flags,
				mode=ya_test_runner.exec.command.RunMode.INTERACTIVE,
			),
			ya_test_runner.test.CommandToResultStep(),
			ya_test_runner.test.ResultStopIfErrorStep(),
			ya_test_runner.exec.step.Run(
				args=[
					'cargo',
					'afl',
					'fuzz',
					'-c',
					'-',
					'-M',
					'main',
					'-i',
					f'./fuzz/inputs-{name}',
					'-o',
					f'{local_ctx.shared.artifacts_dir}/fuzz/{name}',
					'-V',
					str(getattr(ctx.configuration.args, 'fuzz_timeout', 30)),
					'-t',
					'5000',
					f'{TARGET_DIR}/debug/examples/fuzz-{name}',
				],
				mode=ya_test_runner.exec.command.RunMode.INTERACTIVE_TTY,
			),
			ya_test_runner.test.CommandToResultStep(),
			ya_test_runner.test.ResultStopIfErrorStep(),
		]
	)

	if getattr(ctx.configuration.args, 'fuzz_update_corpus', False):
		opt_dir = local_ctx.shared.artifacts_dir.joinpath('fuzz/', f'{name}-opt')

		async def remove_opt_dir(_):
			if opt_dir.exists():
				shutil.rmtree(opt_dir, ignore_errors=True)
			opt_dir.mkdir(parents=True, exist_ok=True)

		inputs_dir = rust_root_dir.joinpath('fuzz', f'inputs-{name}')

		async def remove_inputs_dir(_):
			if inputs_dir.exists():
				shutil.rmtree(inputs_dir, ignore_errors=True)
			inputs_dir.mkdir(parents=True, exist_ok=True)

		steps.append(ya_test_runner.exec.step.PythonFunction(remove_opt_dir))
		steps.append(
			ya_test_runner.exec.step.Run(
				args=[
					'cargo',
					'afl',
					'cmin',
					'-T',
					'all',
					'-o',
					f'{local_ctx.shared.artifacts_dir}/fuzz/{name}-opt',
					'-i',
					f'{local_ctx.shared.artifacts_dir}/fuzz/{name}',
					'--',
					f'{TARGET_DIR}/debug/examples/fuzz-{name}',
				],
				mode=ya_test_runner.exec.command.RunMode.INTERACTIVE,
			)
		)
		steps.extend(
			[
				ya_test_runner.test.CommandToResultStep(),
				ya_test_runner.test.ResultStopIfErrorStep(),
				ya_test_runner.exec.step.PythonFunction(remove_inputs_dir),
			]
		)
		steps.append(
			ya_test_runner.exec.step.Run(
				args=[
					sys.executable,
					f'{local_ctx.shared.root_dir}/runners/genlayer-py-std/fuzz/resave.py',
					opt_dir,
					inputs_dir,
				],
				mode=ya_test_runner.exec.command.RunMode.INTERACTIVE,
			)
		)
		steps.append(CONST_PASSED)

	case = ya_test_runner.test.StepsCase(
		description=desc,
		steps=steps,
	)

	ctx.add_case(case)


local_ctx.plugins['cargo_test'] = cargo_test
local_ctx.plugins['cargo_fuzz'] = cargo_fuzz
