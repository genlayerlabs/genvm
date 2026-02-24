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


class ParserResult(typing.NamedTuple):
	"""Result of create_parser containing main parser and subparsers."""

	parser: argparse.ArgumentParser
	run_parser: argparse.ArgumentParser


def create_parser() -> ParserResult:
	"""Create the command-line argument parser."""
	parser = argparse.ArgumentParser(
		prog='ya-test-runner', description='A test runner utility'
	)

	subparsers = parser.add_subparsers()

	run_parser = subparsers.add_parser('run', help='run tests')
	run_parser.set_defaults(func=workflow_run)

	run_parser.add_argument(
		'--fail-fast',
		action='store_true',
		default=False,
		help='Stop execution after the first test failure',
	)

	default_cpu_count = os.cpu_count() or 1

	run_parser.add_argument(
		'--max-concurrent',
		type=int,
		default=default_cpu_count,
		metavar='N',
		help=f'Maximum number of tests to run concurrently (default: number of CPUs = {default_cpu_count})',
	)

	# 'show' subcommand with nested subcommands
	show_parser = subparsers.add_parser('show', help='show information without running')
	show_subparsers = show_parser.add_subparsers()

	show_plan_parser = show_subparsers.add_parser('plan', help='show execution plan')
	show_plan_parser.set_defaults(func=workflow_plan)

	show_test_parser = show_subparsers.add_parser('test', help='show available tests')
	show_test_parser.set_defaults(func=workflow_list)

	show_services_parser = show_subparsers.add_parser(
		'services', help='show service dependencies'
	)
	show_services_parser.set_defaults(func=workflow_services)

	show_tags_parser = show_subparsers.add_parser('tags', help='show available tags')
	show_tags_parser.set_defaults(func=workflow_tags)

	return ParserResult(parser, run_parser)


from . import const


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

	plan_items = []
	for action in actions:
		if isinstance(action, StartService):
			plan_items.append(f'start service: {action.service.name}')
		elif isinstance(action, StopService):
			plan_items.append(f'stop service: {action.service.name}')
		elif isinstance(action, StartCases):
			case_names = [c.description.name for c in action.cases]
			if action.id in immediate_await_batches:
				# Immediately awaited - use simple "run:" format
				if len(case_names) == 1:
					plan_items.append(f'run: {case_names[0]}')
				else:
					plan_items.append({f'run parallel ({len(case_names)} tests):': case_names})
			else:
				# Deferred await - use "batch N := start" format
				if len(case_names) == 1:
					plan_items.append(f'batch {action.id} := start: {case_names[0]}')
				else:
					plan_items.append(
						{
							f'batch {action.id} := start parallel ({len(case_names)} tests):': case_names
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


def workflow_plan(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	"""Show execution plan without running tests."""
	collection_env = ya_test_runner.stage.collection.run(
		shared_context,
		conf_env,
	)
	shared_context.logger.debug('stage completed', stage='collection')
	collection_env = ya_test_runner.stage.filter.run(
		shared_context,
		collection_env,
	)
	shared_context.logger.debug('stage completed', stage='filter')
	scheduling_env = ya_test_runner.stage.scheduling.run(
		shared_context,
		collection_env,
	)
	shared_context.logger.debug('stage completed', stage='scheduling')

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
		_workflow_run_inner(shared_context, conf_env)
	except KeyboardInterrupt:
		shared_context.logger.warning('Interrupted')
		sys.exit(130)
	finally:
		signal.signal(signal.SIGINT, original_handler)


def _workflow_run_inner(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	collection_env = ya_test_runner.stage.collection.run(
		shared_context,
		conf_env,
	)
	shared_context.logger.debug('stage completed', stage='collection')
	collection_env = ya_test_runner.stage.filter.run(
		shared_context,
		collection_env,
	)
	shared_context.logger.debug('stage completed', stage='filter')
	scheduling_env = ya_test_runner.stage.scheduling.run(
		shared_context,
		collection_env,
	)
	shared_context.logger.debug('stage completed', stage='scheduling')

	execution_env = asyncio.run(
		ya_test_runner.stage.execution.run(
			shared_context,
			scheduling_env,
		)
	)
	shared_context.logger.debug('stage completed', stage='execution')

	success = ya_test_runner.stage.report.run(
		shared_context,
		execution_env,
	)
	shared_context.logger.debug(
		'stage completed',
		stage='report',
		success=success,
	)

	# Run plugin-registered post-run steps
	for step in conf_env.post_run_steps:
		try:
			step(shared_context, execution_env)
		except Exception as e:
			shared_context.logger.error('Post-run step failed', error=str(e))
	shared_context.logger.debug('stage completed', stage='post-run')

	if success:
		sys.exit(0)
	else:
		sys.exit(1)


def workflow_list(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
):
	collection_env = ya_test_runner.stage.collection.run(
		shared_context,
		conf_env,
	)
	shared_context.logger.debug('stage completed', stage='collection')

	collection_env = ya_test_runner.stage.filter.run(
		shared_context,
		collection_env,
	)
	shared_context.logger.debug('stage completed', stage='filter')

	shared_context.printer.put(
		'util stats',
		plugins_count=len(vars(conf_env.plugins)),
		collectors_count=len(conf_env.collectors),
	)

	shared_context.printer.put(
		'available test cases',
		total=len(collection_env.cases),
		cases=[
			{'name': case.description.name, 'tags': case.description.tags}
			for case in collection_env.cases
		],
	)


def workflow_services(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	"""Show service dependencies."""
	collection_env = ya_test_runner.stage.collection.run(
		shared_context,
		conf_env,
	)
	shared_context.logger.debug('stage completed', stage='collection')

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


def workflow_tags(
	shared_context: ya_test_runner.SharedContext,
	conf_env: ya_test_runner.stage.configuration.Env,
) -> None:
	"""Show available tags."""
	collection_env = ya_test_runner.stage.collection.run(
		shared_context,
		conf_env,
	)
	shared_context.logger.debug('stage completed', stage='collection')

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

	base_parser = argparse.ArgumentParser(add_help=False)
	base_parser.add_argument(
		'-C', '--chdir', type=str, help='Change working directory before doing anything'
	)
	base_parser.add_argument(
		'--log-format', choices=['text', 'json'], default='text', help='Log format'
	)
	base_parser.add_argument(
		'--log-level',
		choices=['trace', 'debug', 'info', 'warning', 'error'],
		default='info',
		help='Logging level',
	)

	base_args, remaining_args = base_parser.parse_known_args()

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

	parser_result = create_parser()

	ya_test_runner.stage.filter.add_args(parser_result.parser)

	conf_env = ya_test_runner.stage.configuration.run(
		shared_context,
		parser_result,
		remaining_args,
	)
	shared_context.logger.debug('stage completed', stage='configuration')

	if 'func' not in conf_env.args:
		logger.error('subcommand not given')
		parser_result.parser.print_help()
		sys.exit(1)

	conf_env.args.func(shared_context, conf_env)


if __name__ == '__main__':
	main()
