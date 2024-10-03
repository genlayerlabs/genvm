import collections.abc

import typing
import os
import abc

class AwaitableResult[T]:
	_exc: typing.Optional[Exception]
	_fd: int
	_res: T | None
	def __init__(self, fd: int):
		self._fd = fd
		self._exc = None
		self._res = None
	def __del__(self):
		if self._fd == 0:
			return
		os.close(self._fd)
		self._fd = 0
	def __await__(self) -> typing.Generator[None, None, T]:
		if self._fd == 0:
			if self._exc is not None:
				raise self._exc
			return self._res # type: ignore
		try:
			self._res = self._get_res(self._fd)
			return self._res
		except Exception as e:
			self._exc = e
			raise
		finally:
			self._fd = 0
		# it is intentionally unreachable
		yield
	@abc.abstractmethod
	def _get_res(self, fd: int) -> T: ...

class AwaitableResultStr(AwaitableResult[str]):
	def _get_res(self, fd: int) -> str:
		with os.fdopen(fd, "rt") as f:
			return f.read()

class AwaitableResultBytes(AwaitableResult[str]):
	def _get_res(self, fd: int) -> bytes:
		with os.fdopen(fd, "rb") as f:
			return f.read()

class AwaitableResultMap[T](AwaitableResult[T]):
	def __init__(self, fd: int, fn: collections.abc.Callable[[bytes], T]):
		super().__init__(fd)
		self._fn = fn
	def _get_res(self, fd: int) -> T:
		with os.fdopen(fd, "rb") as f:
			return self._fn(f.read())
