#!/usr/bin/env python3

"""Command-line interface for ya-test-runner."""

import argparse
import asyncio
import signal
import sys, os
import typing
from pathlib import Path

import ya_test_runner
import ya_test_runner.stage
import copy

from . import formatter
from .stage.pipeline import (
	wrap,
	pipe,
	CollectionStage,
	FilterStage,
	SchedulingStage,
	ExecutionStage,
	ReportStage,
)


class _ParserResult(typing.NamedTuple):
	parser: argparse.ArgumentParser
	run_parser: argparse.ArgumentParser
	shared_parser: argparse.ArgumentParser
	filter_parser: argparse.ArgumentParser


def _create_shared_parser() -> argparse.ArgumentParser:
	shared = argparse.ArgumentParser(add_help=False)
	shared.add_argument(
		'-C', '--chdir', type=str, help='Change working directory before doing anything'
	)
	shared.add_argument(
		'--log-format', choices=['text', 'json'], default='text', help='Log format'
	)
	shared.add_argument(
		'--log-level',
		choices=['trace', 'debug', 'info', 'warning', 'error'],
		default='info',
		help='Logging level',
	)
	return shared


def create_parser(shared: argparse.ArgumentParser) -> _ParserResult:
	"""Create the command-line argument parser."""
	filter_parser = argparse.ArgumentParser(add_help=False)
	ya_test_runner.stage.filter.add_args(filter_parser)

	parser = argparse.ArgumentParser(
		prog='ya-test-runner',
		description='A test runner utility',
		parents=[shared],
	)

	subparsers = parser.add_subparsers()

	run_parser = subparsers.add_parser(
		'run', parents=[shared, filter_parser], help='run tests'
	)
	run_parser.set_defaults(func=workflow_run)

	run_parser.add_argument(
		'--fail-fast',
		action='store_true',
		default=False,
		help='Stop execution after the first test failure',
	)

	default_cpu_count = os.cpu_count() or 1

	run_parser.add_argument(
		'--junit-xml',
		type=str,
		default=None,
		metavar='FILE',
		help='Write JUnit XML report to FILE (default: <artifacts_dir>/junit.xml)',
	)

	run_parser.add_argument(
		'--max-concurrent',
		type=int,
		default=default_cpu_count,
		metavar='N',
		help=f'Maximum number of tests to run concurrently (default: number of CPUs = {default_cpu_count})',
	)

	# 'show' subcommand with nested subcommands
	show_parser = subparsers.add_parser(
		'show', parents=[shared], help='show information without running'
	)
	show_subparsers = show_parser.add_subparsers()

	show_plan_parser = show_subparsers.add_parser(
		'plan', parents=[shared, filter_parser], help='show execution plan'
	)
	show_plan_parser.set_defaults(func=workflow_plan)

	show_test_parser = show_subparsers.add_parser(
		'test', parents=[shared, filter_parser], help='show available tests'
	)
	show_test_parser.set_defaults(func=workflow_list)

	show_services_parser = show_subparsers.add_parser(
		'services', parents=[shared], help='show service dependencies'
	)
	show_services_parser.set_defaults(func=workflow_services)

	show_tags_parser = show_subparsers.add_parser(
		'tags', parents=[shared], help='show available tags'
	)
	show_tags_parser.set_defaults(func=workflow_tags)

	return _ParserResult(parser, run_parser, shared, filter_parser)


from . import const
import xml.etree.ElementTree as ET


def _show_execution_plan(
	shared_context: ya_test_runner.SharedContext,
	scheduling_env: ya_test_runner.stage.scheduling.Env,
) -> None:
	"""Display the execution plan."""
	from ya_test_runner.stage.scheduling import (
		StartCases,
		AwaitAllCases,
		StartService,
		StopService,
	)

	# Find batches that are immediately awaited (start followed by await with nothing in between)
	actions = scheduling_env.actions
	immediate_await_batches: set[int] = set()
	for i, action in enumerate(actions):
		if isinstance(action, StartCases):
			if i + 1 < len(actions) and isinstance(actions[i + 1], AwaitAllCases):
				if actions[i + 1].id == action.id:
					immediate_await_batches.add(action.id)

	def _case_info(case: ya_test_runner.test.Case) -> str | dict:
		if case.description.depends_on:
			return {
				'name': case.description.name,
				'depends_on': sorted(case.description.depends_on),
			}
		return case.description.name

	plan_items = []
	for action in actions:
		if isinstance(action, StartService):
			plan_items.append(f'start service: {action.service.name}')
		elif isinstance(action, StopService):
			plan_items.append(f'stop service: {action.service.name}')
		elif isinstance(action, StartCases):
			case_infos = [_case_info(c) for c in action.cases]
			if action.id in immediate_await_batches:
				# Immediately awaited - use simple "run:" format
				if len(case_infos) == 1:
					plan_items.append({'run': case_infos[0]})
				else:
					plan_items.append({f'run parallel ({len(case_infos)} tests):': case_infos})
			else:
				# Deferred await - use "batch N := start" format
				if len(case_infos) == 1:
					plan_items.append({f'batch {action.id} := start': case_infos[0]})
				else:
					plan_items.append(
						{
							f'batch {action.id} := start parallel ({len(case_infos)} tests):': case_infos
						}
					)
		elif isinstance(action, AwaitAllCases):
			# Skip await for immediately awaited batches (already shown as "run")
			if action.id not in immediate_await_batches:
				plan_items.append(f'await batch {action.id}')

	shared_context.printer.put(
		'execution plan',
		total_actions=len(scheduling_env.actions),
		plan=plan_items,
	)


async def workflow_plan(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	"""Show execution plan without running tests."""
	to_scheduling = pipe(
		wrap(CollectionStage()), pipe(wrap(FilterStage()), wrap(SchedulingStage()))
	)
	scheduling_env = await to_scheduling(shared_context, conf_env)
	_show_execution_plan(shared_context, scheduling_env)


def workflow_run(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	# Set up Ctrl+C handling for graceful shutdown
	interrupted = False

	def signal_handler(signum, frame):
		nonlocal interrupted
		if interrupted:
			# Second Ctrl+C, force exit
			shared_context.logger.warning('Received second interrupt, forcing exit')
			sys.exit(130)
		interrupted = True
		shared_context.logger.warning(
			'Received interrupt, stopping gracefully (press Ctrl+C again to force)'
		)
		shared_context.interrupt()

	original_handler = signal.signal(signal.SIGINT, signal_handler)

	try:
		asyncio.run(_workflow_run_inner(shared_context, conf_env))
	except KeyboardInterrupt:
		shared_context.logger.warning('Interrupted')
		sys.exit(130)
	finally:
		signal.signal(signal.SIGINT, original_handler)


def _write_junit_xml(
	path: Path,
	shared: ya_test_runner.SharedContext,
	exec_env: ya_test_runner.stage.execution.Env,
) -> None:
	testsuites = ET.Element('testsuites')
	testsuite = ET.SubElement(testsuites, 'testsuite', name='ya-test-runner')

	total_time = 0.0
	failures = 0
	for record in exec_env.results:
		total_time += record.elapsed_seconds
		attrs = {
			'name': record.name,
			'time': f'{record.elapsed_seconds:.3f}',
		}
		testcase = ET.SubElement(testsuite, 'testcase', **attrs)
		if not record.passed:
			failures += 1
			failure = ET.SubElement(testcase, 'failure', message='Test failed')
			if record.failure_message:
				failure.text = record.failure_message

	testsuite.set('tests', str(len(exec_env.results)))
	testsuite.set('failures', str(failures))
	testsuite.set('time', f'{total_time:.3f}')

	path.parent.mkdir(parents=True, exist_ok=True)
	tree = ET.ElementTree(testsuites)
	ET.indent(tree)
	tree.write(path, xml_declaration=True, encoding='unicode')


async def _workflow_run_inner(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	to_execution = pipe(
		wrap(CollectionStage()),
		pipe(wrap(FilterStage()), pipe(wrap(SchedulingStage()), wrap(ExecutionStage()))),
	)
	execution_env = await to_execution(shared_context, conf_env)

	junit_xml_path = conf_env.args.junit_xml
	if junit_xml_path is None:
		junit_xml_path = shared_context.artifacts_dir / 'junit.xml'
	else:
		junit_xml_path = Path(junit_xml_path)
	conf_env.post_run_steps.append(
		lambda shared, env: _write_junit_xml(junit_xml_path, shared, env)
	)

	success = await wrap(ReportStage())(shared_context, execution_env)

	# Run plugin-registered post-run steps
	for step in conf_env.post_run_steps:
		try:
			step(shared_context, execution_env)
		except Exception as e:
			shared_context.logger.error('Post-run step failed', error=str(e))

	if success:
		sys.exit(0)
	else:
		sys.exit(1)


async def workflow_list(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
):
	to_filter = pipe(wrap(CollectionStage()), wrap(FilterStage()))
	collection_env = await to_filter(shared_context, conf_env)

	shared_context.printer.put(
		'util stats',
		plugins_count=len(vars(conf_env.plugins)),
		collectors_count=len(conf_env.collectors),
	)

	cases_info = []
	for case in collection_env.cases:
		info: dict[str, typing.Any] = {
			'name': case.description.name,
			'tags': case.description.tags,
		}
		if case.description.depends_on:
			info['depends_on'] = sorted(case.description.depends_on)
		cases_info.append(info)

	shared_context.printer.put(
		'available test cases',
		total=len(collection_env.cases),
		cases=cases_info,
	)


async def workflow_services(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	"""Show service dependencies."""
	collection_env = await wrap(CollectionStage())(shared_context, conf_env)

	# Collect all services from cases
	all_services: set[ya_test_runner.stage.collection.Service] = set()
	for case in collection_env.cases:
		for svc in case.description.needed_services:
			all_services.add(svc)
			if svc.depends_on:
				for dep in svc.depends_on:
					all_services.add(dep)

	# Build service info list
	services_info = []
	for svc in sorted(all_services, key=lambda s: s.name):
		info: dict[str, typing.Any] = {'name': svc.name}
		if svc.depends_on:
			info['depends_on'] = [dep.name for dep in svc.depends_on]
		services_info.append(info)

	shared_context.printer.put(
		'services',
		total=len(all_services),
		services=services_info,
	)


async def workflow_tags(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	"""Show available tags."""
	collection_env = await wrap(CollectionStage())(shared_context, conf_env)

	# Collect all tags and count usage
	tag_counts: dict[str, int] = {}
	for case in collection_env.cases:
		for tag in case.description.tags:
			tag_counts[tag] = tag_counts.get(tag, 0) + 1

	# Build tags info sorted by name
	tags_info = [
		{'tag': tag, 'count': count} for tag, count in sorted(tag_counts.items())
	]

	shared_context.printer.put(
		'tags',
		total=len(tag_counts),
		tags=tags_info,
	)


def main() -> None:
	"""
	even before collecting args, we need to collect all suites
	because they may add extra args to the parser
	"""

	shared_parser = _create_shared_parser()

	base_args, remaining_args = shared_parser.parse_known_args()

	stdoutWithLock = formatter.DefaultLockableTextIO(sys.stdout)
	stderrWithLock = formatter.DefaultLockableTextIO(sys.stderr)

	match base_args.log_format:
		case 'text':
			logger = formatter.TextFormatter(stderrWithLock)
			printer = formatter.TextFormatter(stdoutWithLock)
		case 'json':
			logger = formatter.JsonFormatter(stderrWithLock)
			printer = formatter.JsonFormatter(stdoutWithLock)
		case _:
			raise RuntimeError(f'unknown log format: {base_args.log_format}')

	logger.min_level = formatter.Level.from_str(base_args.log_level)

	if base_args.chdir:
		new_cwd = Path(base_args.chdir).absolute()
		logger.trace('changing working directory', new_cwd=new_cwd)
		os.chdir(new_cwd)

	cur_dir = Path('.').absolute()
	while True:
		if cur_dir.joinpath(const.ROOT_FILE_NAME).exists():
			break
		else:
			parent_dir = cur_dir.parent
			if parent_dir == cur_dir:
				raise RuntimeError('.ya-test.py not found in any parent directory')
			cur_dir = parent_dir

	logger.trace('found root directory', root_dir=cur_dir)

	shared_context = ya_test_runner.SharedContext(
		root_dir=cur_dir,
		logger=logger,
		printer=printer,
	)

	for p in shared_context.config.get('extra_python_paths', []):
		extra_path = Path(p)
		if not extra_path.is_absolute():
			extra_path = shared_context.root_dir / extra_path
		extra_path_str = str(extra_path)
		if extra_path_str not in sys.path:
			logger.debug('adding extra python path', path=extra_path)
			sys.path.append(extra_path_str)

	parser_result = create_parser(shared_parser)

	initial_env = ya_test_runner.stage.configuration.InitialEnv(
		parser=parser_result.parser,
		run_parser=parser_result.run_parser,
		shared_parser=parser_result.shared_parser,
		filter_parser=parser_result.filter_parser,
		remaining_args=remaining_args,
	)

	conf_step = wrap(ya_test_runner.stage.pipeline.ConfigurationStage())

	async def foo():
		return await conf_step(shared_context, initial_env)

	conf_env = asyncio.run(foo())

	if 'func' not in conf_env.args:
		logger.error('subcommand not given')
		parser_result.parser.print_help()
		sys.exit(1)

	try:
		func = conf_env.args.func
		if asyncio.iscoroutinefunction(func):
			asyncio.run(func(shared_context, conf_env))
		else:
			func(shared_context, conf_env)
	finally:
		shared_context.watchdog.stop()


if __name__ == '__main__':
	main()
