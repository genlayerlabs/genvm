import json
import os
import shutil
from pathlib import Path
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

default_env = {
	k: v
	for k, v in os.environ.items()
	if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
}

default_env['LLVM_PROFILE_FILE'] = '/dev/null'

default_env['AFL_FUZZER_LOOPCOUNT'] = '20'  # without it no coverage will be written!
default_env['AFL_NO_CFG_FUZZING'] = '1'
default_env['AFL_BENCH_UNTIL_CRASH'] = '1'


def cargo_test(
	ctx: ya_test_runner.stage.collection.Context,
	desc: ya_test_runner.test.Description,
	*,
	rust_root_dir: Path,
):
	desc = desc.with_tags(['rust', 'unit'])._replace(
		console_pool=True,
	)

	test_env = default_env.copy()

	extra_flags = []

	extra_config = rust_root_dir.joinpath('.ya-test-config.json')
	if extra_config.exists():
		extra_conf = json.loads(extra_config.read_text())
		extra_flags.extend(extra_conf.get('cargo_test_flags', []))
		for name in extra_conf.get('keep_env', []):
			if name in os.environ:
				test_env[name] = os.environ[name]

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

	test_env = default_env.copy()

	extra_flags = []

	extra_config = rust_root_dir.joinpath('.ya-test-config.json')
	if extra_config.exists():
		extra_conf = json.loads(extra_config.read_text())
		extra_flags.extend(extra_conf.get('cargo_test_flags', []))

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
					f'{BUILD_DIR}/genvm-testdata-out/fuzz/{name}',
					'-V',
					str(ctx.configuration.args.fuzz_timeout),
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

	if ctx.configuration.args.fuzz_update_corpus:

		async def remove_opt_dir(_):
			opt_dir = BUILD_DIR.joinpath('genvm-testdata-out', 'fuzz/', f'{name}-opt')
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
					f'{BUILD_DIR}/genvm-testdata-out/fuzz/{name}-opt',
					'-i',
					f'{BUILD_DIR}/genvm-testdata-out/fuzz/{name}',
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
					f'{BUILD_DIR}/genvm-testdata-out/fuzz/{name}-opt',
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
