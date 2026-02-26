import json
import os
import shlex
from pathlib import Path

import ya_test_runner
from ya_test_runner.test import Result

local_ctx = ya_test_runner.stage.configuration.current_context()

build_info = json.loads(
	local_ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
)

BUILD_DIR = Path(build_info['build_dir'])


def _get_default_env() -> dict[str, str]:
	return {
		k: v
		for k, v in os.environ.items()
		if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
	}


def parse_version_test(
	ctx: ya_test_runner.stage.collection.Context,
	desc: ya_test_runner.test.Description,
	*,
	wat_file: Path,
	expected_file: Path,
	genvm_bin: Path,
	config_path: Path,
	artifacts_dir: Path,
):
	desc = desc.with_tags(['parse_version'])

	test_env = _get_default_env()

	tmp_dir = artifacts_dir / wat_file.stem
	wasm_file = tmp_dir / f'{wat_file.stem}.wasm'

	expected_output = expected_file.read_text().strip()

	steps: list[ya_test_runner.exec.step.Step] = []
	steps.append(ya_test_runner.exec.step.MkDir(path=tmp_dir))
	steps.append(ya_test_runner.exec.step.SetCwd(path=tmp_dir))

	for k, v in test_env.items():
		steps.append(ya_test_runner.exec.step.SetEnv(key=k, value=v))

	# Compile WAT to WASM
	steps.append(
		ya_test_runner.exec.step.Run(
			args=[
				'wat2wasm',
				'--enable-annotations',
				'-o',
				str(wasm_file),
				str(wat_file),
			],
			mode=ya_test_runner.exec.command.RunMode.SILENT,
		)
	)
	steps.append(ya_test_runner.test.CommandToResultStep())
	steps.append(ya_test_runner.test.ResultStopIfErrorStep())

	# Run genvm parse-version-pattern with wasm piped to stdin
	shell_cmd = (
		f'{shlex.quote(str(genvm_bin))}'
		f' --config {shlex.quote(str(config_path))}'
		f' parse-version-pattern'
		f' < {shlex.quote(str(wasm_file))}'
	)
	steps.append(
		ya_test_runner.exec.step.Run(
			args=['sh', '-c', shell_cmd],
			mode=ya_test_runner.exec.command.RunMode.SILENT,
		)
	)

	# Validate output
	async def validate(previous_results):
		res = previous_results[-1]
		assert isinstance(res, ya_test_runner.exec.command.Result)
		actual = res.stdout
		if res.exit_code != 0:
			return Result(
				passed=False,
				context={
					'reason': 'genvm parse-version-pattern failed',
					'exit_code': res.exit_code,
					'stderr': res.stderr,
				},
				elapsed_seconds=res.elapsed_seconds,
			)
		if actual != expected_output:
			return Result(
				passed=False,
				context={
					'expected': repr(expected_output),
					'actual': repr(actual),
				},
				elapsed_seconds=res.elapsed_seconds,
			)
		return Result(passed=True, context={}, elapsed_seconds=res.elapsed_seconds)

	steps.append(ya_test_runner.exec.step.PythonFunction(validate))

	case = ya_test_runner.test.StepsCase(
		description=desc,
		steps=steps,
	)
	ctx.add_case(case)


local_ctx.plugins['parse_version_test'] = parse_version_test
