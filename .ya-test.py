from pathlib import Path
import ya_test_runner

local_ctx = ya_test_runner.stage.configuration.current_context()

local_ctx.run_parser.add_argument(
	'--fuzz-timeout',
	type=int,
	default=30,
	help='Timeout for each fuzzing run in seconds',
)


local_ctx.run_parser.add_argument(
	'--fuzz-update-corpus',
	default=False,
	action='store_true',
	help='Whether to update the fuzzing corpus',
)

import sys
import ya_test_runner_plugins

local_ctx.shared.logger.trace(
	'import path', path=sys.path, plugins_path=ya_test_runner_plugins.__path__
)
from ya_test_runner_plugins import cargo, pytest, integration, genvm


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


local_ctx.run_parser.add_argument(
	'--genvm-reroute-to',
	type=str,
	default='vTEST',
	help='Reroute GenVM Manager to the specified environment',
)


def collect_integration(ctx: ya_test_runner.stage.collection.Context):
	import json

	# Load build info to find binary paths
	build_info = json.loads(
		ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
	)
	build_dir = Path(build_info['build_dir'])

	tests_output_root = ctx.shared.artifacts_dir.joinpath('integration')
	tests_output_root.mkdir(parents=True, exist_ok=True)

	manager_impl = genvm.ManagerService(
		bin_path=build_dir.joinpath('out', 'bin', 'genvm-modules'),
		reroute_to=ctx.configuration.args.genvm_reroute_to,
		log_path=tests_output_root.joinpath('manager.log'),
		env=ctx.configuration,
	)
	manager_service = ctx.new_service(
		name=f'manager',
		manager=manager_impl,
	)
	manager_port = genvm.get_manager_port(ctx.configuration)
	manager_service.meta = {'port': manager_port}

	# Create webdriver service
	webdriver_impl = ya_test_runner.exec.service.FunctionService(
		lambda: genvm.start_webdriver_service(ctx.configuration)
	)
	webdriver_service = ctx.new_service(
		name=f'webdriver',
		manager=webdriver_impl,
	)

	# This starts Llm and Web modules on the manager
	modules_impl = genvm.ModulesService(
		manager_uri=f'http://localhost:{manager_port}',
	)
	modules_service = ctx.new_service(
		name='modules',
		manager=modules_impl,
		depends_on=[manager_service, webdriver_service],
	)

	# Collect integration tests
	ctx.configuration.plugins.integration_test(
		ctx,
		manager_service=manager_service,
		modules_service=modules_service,
		webdriver_service=webdriver_service,
	)


local_ctx.add_collector(collect_integration)
