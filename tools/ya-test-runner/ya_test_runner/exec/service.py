import abc


class Handle(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def healthy(self) -> bool: ...

	@abc.abstractmethod
	async def interrupt(self) -> None: ...


class Service(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def start(self) -> Handle: ...
