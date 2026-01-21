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

	# 'show' subcommand with nested subcommands
	show_parser = subparsers.add_parser('show', help='show information without running')
	show_subparsers = show_parser.add_subparsers()

	show_plan_parser = show_subparsers.add_parser('plan', help='show execution plan')
	show_plan_parser.set_defaults(func=workflow_plan)

	show_test_parser = show_subparsers.add_parser('test', help='show available tests')
	show_test_parser.set_defaults(func=workflow_list)

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

	plan_items = []
	for action in scheduling_env.actions:
		if isinstance(action, StartService):
			plan_items.append(f'start service: {action.service.name}')
		elif isinstance(action, StopService):
			plan_items.append(f'stop service: {action.service.name}')
		elif isinstance(action, StartCases):
			case_names = [c.description.name for c in action.cases]
			if len(case_names) == 1:
				plan_items.append(f'batch {action.id} := run: {case_names[0]}')
			else:
				plan_items.append(
					{f'batch {action.id} := run parallel ({len(case_names)} tests):': case_names}
				)
		elif isinstance(action, AwaitAllCases):
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

	match base_args.log_format:
		case 'text':
			logger = formatter.TextFormatter(sys.stderr)
			printer = formatter.TextFormatter(sys.stdout)
		case 'json':
			logger = formatter.JsonFormatter(sys.stderr)
			printer = formatter.JsonFormatter(sys.stdout)
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
