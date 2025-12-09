from pathlib import Path
import ya_test_runner

local_ctx = ya_test_runner.stage.configuration.current_context()

local_ctx.parser.add_argument(
	'--fuzz-timeout',
	type=int,
	default=30,
	help='Timeout for each fuzzing run in seconds',
)


local_ctx.parser.add_argument(
	'--fuzz-update-corpus',
	default=False,
	action='store_true',
	help='Whether to update the fuzzing corpus',
)

local_ctx.add_dir('tests')


def collect_rust(ctx: ya_test_runner.stage.collection.Context):
	for t in filter(lambda x: x.name == 'Cargo.toml', ctx.shared.git_files):
		ctx.shared.logger.info('discovered Cargo.toml', path=t)
		rust_root_dir = t.parent
		test_dir = rust_root_dir.joinpath('tests')
		if test_dir.exists():
			ctx.configuration.plugins.cargo_test(
				ctx,
				ya_test_runner.test.Description(
					str(test_dir.relative_to(ctx.shared.root_dir)),
					console_pool=True,
				),
				rust_root_dir=rust_root_dir,
			)

		fuzz_files = list(rust_root_dir.glob('fuzz/*.rs'))
		fuzz_files.sort()
		for fuzz_file in fuzz_files:
			ctx.shared.logger.info('discovered fuzz target', path=fuzz_file)

			name = fuzz_file.relative_to(ctx.shared.root_dir)
			name = f'{name.parent}/{name.stem}'
			ctx.configuration.plugins.cargo_fuzz(
				ctx,
				ya_test_runner.test.Description(
					name,
					console_pool=True,
				),
				rust_root_dir=rust_root_dir,
				name=fuzz_file.stem,
			)


def collect_poetry(ctx: ya_test_runner.stage.collection.Context):
	p = ctx.shared.root_dir.joinpath('runners', 'genlayer-py-std')
	ctx.configuration.plugins.pytest(
		ctx,
		ya_test_runner.test.Description(
			'runners/genlayer-py-std/test',
		),
		poetry_root_dir=p,
	)

	fuzz_files = list(p.glob('fuzz/src/*.py'))
	fuzz_files.sort()
	for fuzz_file in fuzz_files:
		name = fuzz_file.relative_to(ctx.shared.root_dir)
		name = f'{name.parent}/{name.stem}'
		continue  # for now let's disable it
		ctx.configuration.plugins.py_fuzz(
			ctx,
			ya_test_runner.test.Description(
				name,
			),
			poetry_root_dir=p,
			name=fuzz_file.stem,
		)


local_ctx.add_collector(collect_rust)
local_ctx.add_collector(collect_poetry)
