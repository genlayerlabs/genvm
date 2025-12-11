import asyncio
from dataclasses import dataclass
import typing
import ya_test_runner
from ya_test_runner import SharedContext

from .scheduling import Env as SchedulingEnv, StartCases, AwaitAllCases


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
	success = False
	context = {}
	try:
		ctx.shared.logger.debug(
			'Running test case',
			case_name=case.description.name,
		)
		# Simulate test execution with sleep
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
		ctx.shared.logger.info(
			'Completed test case',
			case_name=case.description.name,
		)
		if success:
			ctx.success_count += 1
	except Exception as e:
		context['exception'] = e
		ctx.shared.logger.error(
			'Internal exception',
			case_name=case.description.name,
			error=e,
			**context,
		)
	finally:
		if not success:
			ctx.failed.append(case.description.name)


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

	for action in collection_env.actions:
		if isinstance(action, StartCases):
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

	return Env(
		success_count=ctx.success_count,
		failed=ctx.failed,
	)
