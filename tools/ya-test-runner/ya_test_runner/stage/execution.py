import asyncio
from dataclasses import dataclass, field
import typing
import ya_test_runner
from ya_test_runner import SharedContext
from ya_test_runner.formatter import FORMATTING_MUTEX

from .scheduling import (
	Env as SchedulingEnv,
	StartCases,
	AwaitAllCases,
	StartService,
	StopService,
)
from .collection import Service
from ya_test_runner.exec.service import Handle


class _Colors:
	"""Terminal color constants."""

	OKGREEN = '\033[92m'
	FAIL = '\033[91m'
	WARNING = '\033[93m'
	ENDC = '\033[0m'


class Env(typing.NamedTuple):
	success_count: int
	failed: list[str]


@dataclass
class _ExecutionContext:
	shared: SharedContext
	failed: list[str]
	should_stop: asyncio.Event
	success_count: int = 0
	skipped: int = 0
	running_services: dict[str, Handle] = field(default_factory=dict)


def _print_test_result(
	ctx: _ExecutionContext,
	case_name: str,
	passed: bool,
	elapsed: float,
	context: dict[str, typing.Any] | None = None,
) -> None:
	"""Print test result immediately with colors using the formatter."""
	with FORMATTING_MUTEX:
		if passed:
			sign = f'{_Colors.OKGREEN}✓{_Colors.ENDC}'
			category = 'pass'
		else:
			sign = f'{_Colors.FAIL}✗{_Colors.ENDC}'
			category = 'fail'

		elapsed_str = f'{elapsed:.3f}s'

		# Build the output dict for failed tests
		output_kv = {}
		if not passed and context:
			if 'reason' in context:
				output_kv['reason'] = context['reason']
			if 'exception' in context:
				output_kv['exception'] = context['exception']
			if 'stdout' in context and context['stdout']:
				output_kv['stdout'] = context['stdout']
			if 'stderr' in context and context['stderr']:
				output_kv['stderr'] = context['stderr']

		ctx.shared.printer.put(
			f'{sign} {case_name} in {elapsed_str}',
			**output_kv,
		)


class _CountDownLatch:
	def __init__(self, count: int):
		self._count = count
		self._event = asyncio.Event()
		if self._count == 0:
			self._event.set()

	def decrement(self):
		self._count -= 1
		if self._count == 0:
			self._event.set()

	async def wait(self):
		await self._event.wait()


async def _start_service(ctx: _ExecutionContext, service: Service) -> None:
	"""Start a service and track its handle."""
	ctx.shared.logger.info('Starting service', service_name=service.name)
	try:
		handle = await service.manager.start()
		ctx.running_services[service.name] = handle
		ctx.shared.logger.info('Service started', service_name=service.name)
	except Exception as e:
		ctx.shared.logger.error(
			'Failed to start service',
			service_name=service.name,
			error=e,
		)
		raise


async def _stop_service(ctx: _ExecutionContext, service: Service) -> None:
	"""Stop a running service."""
	handle = ctx.running_services.pop(service.name, None)
	if handle is not None:
		ctx.shared.logger.info('Stopping service', service_name=service.name)
		try:
			await handle.interrupt()
			ctx.shared.logger.info('Service stopped', service_name=service.name)
		except Exception as e:
			ctx.shared.logger.error(
				'Failed to stop service cleanly',
				service_name=service.name,
				error=e,
			)


async def _stop_all_services(ctx: _ExecutionContext) -> None:
	"""Stop all running services (cleanup)."""
	service_names = list(ctx.running_services.keys())
	for name in service_names:
		handle = ctx.running_services.pop(name, None)
		if handle is not None:
			ctx.shared.logger.info('Stopping service (cleanup)', service_name=name)
			try:
				await handle.interrupt()
			except Exception as e:
				ctx.shared.logger.error(
					'Failed to stop service during cleanup',
					service_name=name,
					error=e,
				)


async def _run_case(
	ctx: _ExecutionContext, case: ya_test_runner.test.Case, latch: _CountDownLatch
):
	try:
		if ctx.should_stop.is_set():
			return
		await _run_case_locked(ctx, case)
	finally:
		latch.decrement()


async def _run_case_locked(ctx: _ExecutionContext, case: ya_test_runner.test.Case):
	import time

	success = False
	context: dict[str, typing.Any] = {}
	start_time = time.monotonic()
	elapsed = 0.0

	try:
		ctx.shared.logger.debug(
			'Running test case',
			case_name=case.description.name,
		)
		steps = await case.into_steps()
		context['raw_steps'] = ya_test_runner.exec.step.dump_steps(steps)
		steps = ya_test_runner.exec.step.optimize_steps(steps)
		context['steps'] = ya_test_runner.exec.step.dump_steps(steps)
		try:
			res = await ya_test_runner.exec.step.run_steps(ctx.shared, steps)
			context['all_res'] = res
			test_case_result = res[-1]
			del context['all_res']
		except ya_test_runner.test.FinishedEarlyException as e:
			test_case_result = e.result
		context['raw_result'] = test_case_result
		assert isinstance(test_case_result, ya_test_runner.test.Result)
		success = test_case_result.passed
		elapsed = time.monotonic() - start_time

		# Merge test result context into our context
		if test_case_result.context:
			context.update(test_case_result.context)

		if success:
			ctx.success_count += 1
	except Exception as e:
		elapsed = time.monotonic() - start_time
		context['exception'] = e
		ctx.shared.logger.error(
			'Internal exception',
			case_name=case.description.name,
			error=e,
		)
	finally:
		if not success:
			ctx.failed.append(case.description.name)

		# Print result immediately
		_print_test_result(
			ctx,
			case.description.name,
			success,
			elapsed,
			context if not success else None,
		)


_background_tasks: set[asyncio.Task] = set()


def _spawn_background_task(coro: typing.Coroutine) -> None:
	task = asyncio.create_task(coro)
	_background_tasks.add(task)

	task.add_done_callback(_background_tasks.discard)


async def _run_cases(
	ctx: _ExecutionContext, cases: list[ya_test_runner.test.Case], latch: _CountDownLatch
):
	for case in cases:
		_spawn_background_task(_run_case(ctx, case, latch))


async def run(shared: SharedContext, collection_env: SchedulingEnv) -> Env:
	awaiters: dict[int, _CountDownLatch] = {}

	should_stop = asyncio.Event()

	ctx = _ExecutionContext(
		shared=shared,
		failed=[],
		should_stop=should_stop,
	)

	try:
		for action in collection_env.actions:
			if isinstance(action, StartService):
				await _start_service(ctx, action.service)
			elif isinstance(action, StopService):
				await _stop_service(ctx, action.service)
			elif isinstance(action, StartCases):
				awaiters[action.id] = _CountDownLatch(len(action.cases))
				_spawn_background_task(_run_cases(ctx, action.cases, awaiters[action.id]))
			elif isinstance(action, AwaitAllCases):
				shared.logger.debug(
					'Awaiting completion of test cases',
					id=action.id,
				)
				await awaiters[action.id].wait()
				shared.logger.debug(
					'All test cases completed',
					id=action.id,
				)
			else:
				raise ValueError(f'Unknown action type: {type(action)}')
	finally:
		# Always cleanup services on exit
		await _stop_all_services(ctx)

	return Env(
		success_count=ctx.success_count,
		failed=ctx.failed,
	)
