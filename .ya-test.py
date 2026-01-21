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


def collect_integration(ctx: ya_test_runner.stage.collection.Context):
	import json

	# Import docker module for service classes
	from tests.plugins.docker import ManagerService, ModulesService, WebdriverService

	# Load build info to find binary paths
	build_info = json.loads(
		ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
	)
	build_dir = Path(build_info['build_dir'])

	tests_output_root = build_dir.joinpath('genvm-testdata-out')
	tests_output_root.mkdir(parents=True, exist_ok=True)

	# Create manager service with semaphore
	manager_port = 3999
	manager_sem = ctx.new_semaphore(f'manager-port-{manager_port}', limit=1)
	manager_impl = ManagerService(
		bin_path=build_dir.joinpath('out', 'bin', 'genvm-modules'),
		port=manager_port,
		reuse_existing=True,  # Allow reusing pre-started manager during development
		reroute_to='vTEST',
		log_path=tests_output_root.joinpath('manager.log'),
	)
	manager_service = ctx.new_service(
		name=f'manager-{manager_port}',
		sems=[(manager_sem, 1)],
		manager=manager_impl,
	)

	# Create modules service (depends on manager)
	# This starts Llm and Web modules on the manager
	modules_impl = ModulesService(
		manager_uri=f'http://localhost:{manager_port}',
	)
	modules_service = ctx.new_service(
		name='modules',
		sems=[],  # No semaphores - modules don't conflict with anything
		manager=modules_impl,
		depends_on=[manager_service],  # Must start after manager
	)

	# Create webdriver service with semaphore (optional, for web tests)
	webdriver_port = 4444
	webdriver_sem = ctx.new_semaphore(f'webdriver-port-{webdriver_port}', limit=1)
	webdriver_impl = WebdriverService(
		context_dir=ctx.shared.root_dir.joinpath('modules', 'webdriver'),
		port=webdriver_port,
	)
	webdriver_service = ctx.new_service(
		name=f'webdriver-{webdriver_port}',
		sems=[(webdriver_sem, 1)],
		manager=webdriver_impl,
	)

	# Collect integration tests
	ctx.configuration.plugins.integration_test(
		ctx,
		manager_service=manager_service,
		modules_service=modules_service,
		webdriver_service=webdriver_service,
	)


local_ctx.add_collector(collect_integration)
