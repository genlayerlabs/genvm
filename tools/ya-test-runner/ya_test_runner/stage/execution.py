import asyncio
from dataclasses import dataclass, field, replace
import typing
import ya_test_runner
from ya_test_runner import SharedContext
from ya_test_runner import formatter
from ya_test_runner.formatter import (
	FORMATTING_MUTEX,
	DefaultLockableTextIO,
	JsonFormatter,
	Level,
)

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


class _MultiFormatter(formatter.Formatter):
	"""Fan a single log stream out to several formatters (e.g. stderr + file)."""

	def __init__(self, children: typing.Iterable[formatter.Formatter]):
		self._children = list(children)

	def accepts(self, level: Level) -> bool:
		return any(c.accepts(level) for c in self._children)

	def dump(self, level: Level, message: str, **kw) -> None:
		for child in self._children:
			if child.accepts(level):
				child.dump(level, message, **kw)


class TestRecord(typing.NamedTuple):
	name: str
	passed: bool
	elapsed_seconds: float
	failure_message: str | None


class Env(typing.NamedTuple):
	success_count: int
	failed: list[str]
	results: list[TestRecord]


from collections import deque


class MultiSemaphore:
	def __init__(self, value: int):
		self._value = value
		self.max_value = value
		self._waiters: deque[tuple[int, asyncio.Future]] = deque()
		self._lock = asyncio.Lock()

	async def acquire(self, n: int = 1) -> None:
		async with self._lock:
			if self._value >= n and len(self._waiters) == 0:
				self._value -= n
				return
			fut = asyncio.get_event_loop().create_future()
			self._waiters.append((n, fut))
		await fut

	def release(self, n: int = 1) -> None:
		self._value += n
		self._wake_waiters()

	def _wake_waiters(self) -> None:
		while True:
			if len(self._waiters) == 0:
				return
			n, fut = self._waiters.popleft()
			if self._value >= n:
				if not fut.done():
					self._value -= n
					fut.set_result(None)
				continue
			self._waiters.appendleft((n, fut))
			return


@dataclass
class _ExecutionContext:
	shared: SharedContext
	failed: list[str]
	results: list[TestRecord]
	should_stop: asyncio.Event
	semaphore: MultiSemaphore
	fail_fast: bool = False
	success_count: int = 0
	skipped: int = 0
	running_services: dict[str, Handle] = field(default_factory=dict)
	# Per-test completion tracking for test-to-test dependencies
	test_completed: dict[str, asyncio.Event] = field(default_factory=dict)
	test_passed: dict[str, bool] = field(default_factory=dict)


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
			output_kv.update(context)

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
		await handle.await_startup()
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


async def _await_test_dependencies(
	ctx: _ExecutionContext, case: ya_test_runner.test.Case
) -> list[str]:
	"""
	Wait for all test dependencies to complete.
	Returns list of failed dependency names (empty if all passed or were absent).
	Dependencies not in the current test set (e.g. filtered out) are ignored.
	"""
	failed_deps: list[str] = []
	for dep_name in case.description.depends_on:
		event = ctx.test_completed.get(dep_name)
		if event is None:
			# Dependency not in current test set (filtered out), treat as satisfied
			continue
		await event.wait()
		if not ctx.test_passed.get(dep_name, False):
			failed_deps.append(dep_name)
	return failed_deps


async def _run_case(
	ctx: _ExecutionContext, case: ya_test_runner.test.Case, latch: _CountDownLatch
):
	name = case.description.name
	try:
		# Wait for test dependencies before acquiring semaphore
		if case.description.depends_on:
			failed_deps = await _await_test_dependencies(ctx, case)
			if failed_deps:
				# Skip this test: dependency failed
				ctx.skipped += 1
				ctx.failed.append(name)
				if ctx.fail_fast:
					ctx.should_stop.set()
				if not case.hidden:
					with FORMATTING_MUTEX:
						deps_str = ', '.join(failed_deps)
						ctx.shared.printer.put(
							f'{_Colors.WARNING}⊘{_Colors.ENDC} {name} skipped (dependency failed: {deps_str})',
						)
				return

		permits = 1
		if case.description.console_pool:
			permits = ctx.semaphore.max_value
		await ctx.semaphore.acquire(permits)
		try:
			if ctx.should_stop.is_set():
				return
			await _run_case_locked(ctx, case)
		finally:
			ctx.semaphore.release(permits)
	finally:
		# Signal completion (pass or fail) so dependents can proceed
		ctx.test_passed.setdefault(name, name not in ctx.failed)
		event = ctx.test_completed.get(name)
		if event is not None:
			event.set()
		latch.decrement()


async def _warn_not_finished(
	ctx: _ExecutionContext, case_name: str, start_time: float
) -> None:
	"""Print a warning every minute while a test is still running."""
	import time

	while True:
		await asyncio.sleep(60)
		elapsed = time.monotonic() - start_time
		ctx.shared.logger.warning(
			'Test still running',
			case_name=case_name,
			elapsed_seconds=round(elapsed, 1),
		)


def _open_test_log(
	ctx: _ExecutionContext, case: ya_test_runner.test.Case
) -> tuple[SharedContext, JsonFormatter | None, typing.IO[str] | None]:
	"""
	Create the per-test artifact folder and a ``SharedContext`` whose logs and
	command scripts are routed (one JSON object per line) into
	``<artifacts>/cases/<name>/log.ytr.log`` (in addition to the global stderr
	logger). Runner-owned files use the ``.ytr.`` infix so they do not clash
	with the test's own artifacts sharing the directory. Falls back to the
	global context if the folder cannot be created.
	"""
	log_dir = ctx.shared.case_dir_for(case.description.name)
	try:
		log_dir.mkdir(parents=True, exist_ok=True)
		log_file = (log_dir / 'log.ytr.log').open('w', encoding='utf-8')
	except OSError as e:
		ctx.shared.logger.error(
			'Failed to create test log directory',
			case_name=case.description.name,
			error=e,
		)
		return ctx.shared, None, None

	file_fmt = JsonFormatter(DefaultLockableTextIO(log_file), min_level=Level.TRACE)
	run_shared = replace(
		ctx.shared,
		logger=_MultiFormatter([ctx.shared.logger, file_fmt]),
		printer=file_fmt,
		case_dir=log_dir,
	)
	return run_shared, file_fmt, log_file


async def _run_case_locked(ctx: _ExecutionContext, case: ya_test_runner.test.Case):
	import time

	success = False
	context: dict[str, typing.Any] = {}
	start_time = time.monotonic()
	elapsed = 0.0

	run_shared, file_fmt, log_file = _open_test_log(ctx, case)

	warn_task: asyncio.Task | None = None
	if not case.description.console_pool:
		warn_task = asyncio.create_task(
			_warn_not_finished(ctx, case.description.name, start_time)
		)

	try:
		run_shared.logger.debug(
			'Running test case',
			case_name=case.description.name,
		)
		steps = await case.into_steps()
		context['raw_steps'] = ya_test_runner.exec.step.dump_steps(steps)
		steps = ya_test_runner.exec.step.optimize_steps(steps)
		context['steps'] = ya_test_runner.exec.step.dump_steps(steps)
		try:
			res = await ya_test_runner.exec.step.run_steps(run_shared, steps)
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

		if success and not case.hidden:
			ctx.success_count += 1
	except Exception as e:
		elapsed = time.monotonic() - start_time
		context['exception'] = e
		run_shared.logger.error(
			'Internal exception',
			case_name=case.description.name,
			error=e,
		)
	finally:
		if warn_task is not None:
			warn_task.cancel()
			try:
				await warn_task
			except asyncio.CancelledError:
				pass

		# Mirror the full result (with context) into the per-test log file
		if file_fmt is not None:
			sign = 'PASS' if success else 'FAIL'
			file_fmt.put(
				f'{sign} {case.description.name} in {elapsed:.3f}s',
				**context,
			)
		if log_file is not None:
			log_file.close()

		if not success:
			ctx.failed.append(case.description.name)
			if ctx.fail_fast:
				ctx.should_stop.set()

		failure_message = None
		if not success:
			if 'exception' in context:
				failure_message = str(context['exception'])
			elif 'stderr' in context:
				failure_message = str(context['stderr'])

		ctx.results.append(
			TestRecord(
				name=case.description.name,
				passed=success,
				elapsed_seconds=elapsed,
				failure_message=failure_message,
			)
		)

		# Print result (skip hidden successes)
		if not (case.hidden and success):
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

	# Get fail_fast from args, default to False if not present
	fail_fast = getattr(collection_env.args, 'fail_fast', False)

	# Collect all test names to create completion events
	all_test_names: set[str] = set()
	for action in collection_env.actions:
		if isinstance(action, StartCases):
			for case in action.cases:
				all_test_names.add(case.description.name)

	test_completed = {name: asyncio.Event() for name in all_test_names}

	ctx = _ExecutionContext(
		shared=shared,
		failed=[],
		results=[],
		should_stop=should_stop,
		fail_fast=fail_fast,
		semaphore=MultiSemaphore(collection_env.args.max_concurrent),
		test_completed=test_completed,
	)

	# Check for interruption periodically
	async def check_interruption():
		while not should_stop.is_set():
			if shared.is_interrupted:
				shared.logger.warning('Execution interrupted')
				should_stop.set()
				return
			await asyncio.sleep(0.1)

	interrupt_checker = asyncio.create_task(check_interruption())

	try:
		for action in collection_env.actions:
			if should_stop.is_set():
				shared.logger.warning('Stopping execution early: awaiting test cases')

				for aw in awaiters.values():
					await aw.wait()
				break
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
		# Stop the interrupt checker
		interrupt_checker.cancel()
		try:
			await interrupt_checker
		except asyncio.CancelledError:
			pass
		# Always cleanup services on exit
		await _stop_all_services(ctx)

	return Env(
		success_count=ctx.success_count,
		failed=ctx.failed,
		results=ctx.results,
	)
