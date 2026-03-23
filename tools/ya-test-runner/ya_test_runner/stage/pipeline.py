import typing
import time
import abc
from ya_test_runner import SharedContext


class Stage[A, R](metaclass=abc.ABCMeta):
	@property
	@abc.abstractmethod
	def name(self) -> str: ...

	@property
	def is_slow(self) -> bool:
		return False

	@abc.abstractmethod
	async def run(self, ctx: SharedContext, arg: A, /) -> R: ...


type RunStage[A, R] = typing.Callable[[SharedContext, A], typing.Awaitable[R]]


def wrap[A, R](stage: Stage[A, R]) -> RunStage[A, R]:
	async def run(ctx: SharedContext, arg: A) -> R:
		ctx.logger.trace(f'stage start', name=stage.name)
		start_t = time.monotonic()
		res = await stage.run(ctx, arg)
		end_t = time.monotonic()
		delta_t = end_t - start_t
		ctx.logger.trace(f'stage end', name=stage.name, elapsed=delta_t)
		if delta_t > 1.0 and not stage.is_slow:
			ctx.logger.warning(f'stage is slow', name=stage.name, elapsed=delta_t)

		return res

	return run


def pipe[A, B, R](f: RunStage[A, B], g: RunStage[B, R]) -> RunStage[A, R]:
	async def run(ctx: SharedContext, input: A) -> R:
		intermediate = await f(ctx, input)
		return await g(ctx, intermediate)

	return run


import ya_test_runner
from . import configuration, collection, filter, scheduling, execution, report


class ConfigurationStage(Stage[configuration.InitialEnv, configuration.Env]):
	@property
	def name(self) -> str:
		return 'configuration'

	async def run(
		self, ctx: SharedContext, arg: configuration.InitialEnv
	) -> configuration.Env:
		return configuration.run(ctx, arg)


class CollectionStage(Stage[configuration.Env, collection.Env]):
	@property
	def name(self) -> str:
		return 'collection'

	async def run(self, ctx: SharedContext, arg: configuration.Env) -> collection.Env:
		return collection.run(ctx, arg)


class FilterStage(Stage[collection.Env, filter.Env]):
	@property
	def name(self) -> str:
		return 'filter'

	async def run(self, ctx: SharedContext, arg: collection.Env) -> filter.Env:
		return filter.run(ctx, arg)


class SchedulingStage(Stage[filter.Env, scheduling.Env]):
	@property
	def name(self) -> str:
		return 'scheduling'

	async def run(self, ctx: SharedContext, arg: filter.Env) -> scheduling.Env:
		return scheduling.run(ctx, arg)


class ExecutionStage(Stage[scheduling.Env, execution.Env]):
	@property
	def name(self) -> str:
		return 'execution'

	async def run(self, ctx: SharedContext, arg: scheduling.Env) -> execution.Env:
		return await execution.run(ctx, arg)


class ReportStage(Stage[execution.Env, bool]):
	@property
	def name(self) -> str:
		return 'report'

	async def run(self, ctx: SharedContext, arg: execution.Env) -> bool:
		return report.run(ctx, arg)
