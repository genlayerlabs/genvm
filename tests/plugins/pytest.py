import os
from pathlib import Path
import ya_test_runner

default_env = {
	k: v
	for k, v in os.environ.items()
	if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
}

default_env['AFL_FUZZER_LOOPCOUNT'] = '20'  # without it no coverage will be written!
default_env['AFL_NO_CFG_FUZZING'] = '1'
default_env['AFL_BENCH_UNTIL_CRASH'] = '1'
default_env.pop('VIRTUAL_ENV', None)

local_ctx = ya_test_runner.stage.configuration.current_context()

cargo_target_dir = local_ctx.shared.root_dir.joinpath(
	'build', 'ya-build', 'rust-target'
)


def pytest(
	ctx: ya_test_runner.stage.collection.Context,
	desc: ya_test_runner.test.Description,
	*,
	poetry_root_dir: Path,
):
	desc = desc.with_tags(['python', 'unit'])._replace(console_pool=True)
	case = ya_test_runner.test.SimpleCommandCase(
		description=desc,
		command=[
			'poetry',
			'run',
			'--',
			'pytest',
			'--color=yes',
		],
		cwd=poetry_root_dir,
		env=default_env,
	)

	ctx.add_case(case)


# poetry run -- py-afl-fuzz -i "fuzz/inputs/$name" -o "fuzz/outputs/$name" -V "$DURATION" -- "fuzz/src/$name.py"


def py_fuzz(
	ctx: ya_test_runner.stage.collection.Context,
	desc: ya_test_runner.test.Description,
	*,
	poetry_root_dir: Path,
	name: str,
):
	desc = desc.with_tags(['python', 'fuzz'])._replace(
		console_pool=True,
	)
	inputs_dir = poetry_root_dir.joinpath('fuzz', 'inputs', name)
	outputs_dir = poetry_root_dir.joinpath('fuzz', 'outputs', name)
	src_file = poetry_root_dir.joinpath('fuzz', 'src', f'{name}.py')

	case = ya_test_runner.test.SimpleCommandCase(
		description=desc,
		command=[
			'poetry',
			'run',
			'--',
			'py-afl-fuzz',
			'-i',
			inputs_dir,
			'-o',
			outputs_dir,
			'-V',
			str(ctx.configuration.args.fuzz_timeout),
			'--',
			src_file,
		],
		cwd=poetry_root_dir,
		env=default_env,
		mode=ya_test_runner.exec.command.RunMode.INTERACTIVE,
	)

	ctx.add_case(case)


local_ctx.plugins['pytest'] = pytest
local_ctx.plugins['py_fuzz'] = py_fuzz
