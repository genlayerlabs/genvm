import abc
import asyncio
import typing


class Handle(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def healthy(self) -> bool: ...

	@abc.abstractmethod
	async def interrupt(self) -> None: ...

	async def await_startup(self) -> typing.Self:
		for i in range(10):
			if await self.healthy():
				return self
			await asyncio.sleep(1)
		await self.interrupt()
		raise RuntimeError('Service failed to become healthy')


class Service(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def start(self) -> Handle: ...


class FunctionService(Service):
	def __init__(self, start_func: typing.Callable[[], typing.Awaitable[Handle]]):
		self._start_func = start_func

	async def start(self) -> Handle:
		return await self._start_func()
