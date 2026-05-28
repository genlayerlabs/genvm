import json
from pathlib import Path
import ya_test_runner

local_ctx = ya_test_runner.stage.configuration.current_context()

_info_path = local_ctx.shared.root_dir / 'build' / 'info.json'
if not _info_path.exists():
	local_ctx.shared.logger.warning('build/info.json not found, generating default')
	_build_dir = local_ctx.shared.root_dir / 'build'
	_build_dir.mkdir(parents=True, exist_ok=True)
	_info_path.write_text(
		json.dumps(
			{
				'coverage_dir': str(_build_dir / 'cov'),
				'build_dir': str(_build_dir),
				'rust_target_dir': str(_build_dir / 'ya-build' / 'rust-target'),
			},
			indent=2,
		)
		+ '\n'
	)

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
from ya_test_runner_plugins import (
	cargo,
	pytest,
	integration,
	genvm,
	parse_version,
	permits,
)


def collect_rust(ctx: ya_test_runner.stage.collection.Context):
	for t in filter(lambda x: x.name == 'Cargo.toml', ctx.shared.git_files):
		ctx.shared.logger.debug('discovered Cargo.toml', path=t)
		rust_root_dir = t.parent

		ctx.configuration.plugins.cargo_test(
			ctx,
			rust_root_dir=rust_root_dir,
		)

		fuzz_files = list(rust_root_dir.glob('fuzz/*.rs'))
		fuzz_files.sort()
		for fuzz_file in fuzz_files:
			ctx.shared.logger.debug('discovered fuzz target', path=fuzz_file)

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

local_ctx.run_parser.add_argument(
	'--no-manager',
	default=False,
	action='store_true',
	help='Do not start manager, modules, or webdriver services (assumes manager is already running)',
)

local_ctx.run_parser.add_argument(
	'--no-webdriver',
	default=False,
	action='store_true',
	help='Do not start the webdriver service (assumes an existing webdriver is reachable on the standard port)',
)


def collect_integration(ctx: ya_test_runner.stage.collection.Context):
	# Load build info to find binary paths
	build_info = json.loads(
		ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
	)
	build_dir = Path(build_info['build_dir'])

	tests_output_root = ctx.shared.artifacts_dir.joinpath('integration')
	tests_output_root.mkdir(parents=True, exist_ok=True)

	# for non-run
	reroute_to = getattr(ctx.configuration.args, 'genvm_reroute_to', 'vTEST')
	no_manager = getattr(ctx.configuration.args, 'no_manager', False)
	no_webdriver = getattr(ctx.configuration.args, 'no_webdriver', False)

	manager_port = genvm.get_manager_port(ctx.configuration)

	if no_manager:
		manager_impl = genvm.ExternalManagerService(port=manager_port)
		webdriver_impl = genvm.NoOpService()
		modules_impl = genvm.NoOpService()
	else:
		manager_impl = genvm.ManagerService(
			bin_path=build_dir.joinpath('out', 'bin', 'genvm-modules'),
			reroute_to=reroute_to,
			log_path=tests_output_root.joinpath('manager.log'),
			env=ctx.configuration,
		)
		# Create webdriver service
		if no_webdriver:
			webdriver_impl = genvm.NoOpService()
		else:
			webdriver_impl = ya_test_runner.exec.service.FunctionService(
				lambda: genvm.start_webdriver_service(ctx.configuration)
			)
		# This starts Llm and Web modules on the manager
		modules_impl = genvm.ModulesService(
			manager_uri=f'http://localhost:{manager_port}',
		)

	manager_service = ctx.new_service(
		name=f'manager',
		manager=manager_impl,
	)
	manager_service.meta = {'port': manager_port}

	webdriver_service = ctx.new_service(
		name=f'webdriver',
		manager=webdriver_impl,
	)

	modules_service = ctx.new_service(
		name='modules',
		manager=modules_impl,
		depends_on=[] if no_manager else [manager_service, webdriver_service],
	)

	# Collect integration tests
	ctx.configuration.plugins.integration_test(
		ctx,
		manager_service=manager_service,
		modules_service=modules_service,
		webdriver_service=webdriver_service,
	)

	ctx.configuration.plugins.permits_test(
		ctx,
		manager_service=manager_service,
	)


local_ctx.add_collector(collect_integration)


def collect_parse_version(ctx: ya_test_runner.stage.collection.Context):
	build_info = json.loads(
		ctx.shared.root_dir.joinpath('build', 'info.json').read_text()
	)
	build_dir = Path(build_info['build_dir'])

	reroute_to = getattr(ctx.configuration.args, 'genvm_reroute_to', 'vTEST')
	genvm_bin = build_dir / 'out' / 'executor' / reroute_to / 'bin' / 'genvm'
	config_path = build_dir / 'out' / 'executor' / reroute_to / 'config' / 'genvm.yaml'

	artifacts_dir = ctx.shared.artifacts_dir / 'parse_version'
	artifacts_dir.mkdir(parents=True, exist_ok=True)

	test_dir = ctx.shared.root_dir / 'tests' / 'cases' / 'stable' / 'parse_version'
	wat_files = sorted(test_dir.glob('*.wat'))

	for wat_file in wat_files:
		expected_file = wat_file.with_suffix('.expected')
		if not expected_file.exists():
			ctx.shared.logger.warning(
				'no .expected file for WAT test', wat_file=str(wat_file)
			)
			continue

		name = f'tests/cases/stable/parse_version/{wat_file.stem}'
		ctx.configuration.plugins.parse_version_test(
			ctx,
			ya_test_runner.test.Description(name),
			wat_file=wat_file,
			expected_file=expected_file,
			genvm_bin=genvm_bin,
			config_path=config_path,
			artifacts_dir=artifacts_dir,
		)


local_ctx.add_collector(collect_parse_version)
