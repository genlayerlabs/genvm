import typing
import genlayer.std._wasi as wasi
import genlayer.py.calldata as calldata
from ...py.types import Rollback, Lazy
import collections.abc
import os
from ..advanced import ContractReturn, ContractError
from .result_codes import ResultCode


def decode_sub_vm_result_retn(
	data: collections.abc.Buffer,
) -> ContractReturn | Rollback | ContractError:
	mem = memoryview(data)
	if mem[0] == ResultCode.ROLLBACK:
		return Rollback(str(mem[1:], encoding='utf8'))
	if mem[0] == ResultCode.RETURN:
		return ContractReturn(calldata.decode(mem[1:]))
	if mem[0] == ResultCode.CONTRACT_ERROR:
		return ContractError(str(mem[1:], encoding='utf8'))
	assert False, f'unknown type {mem[0]}'


def decode_sub_vm_result(data: collections.abc.Buffer) -> typing.Any:
	dat = decode_sub_vm_result_retn(data)
	if isinstance(dat, Rollback):
		raise dat
	assert isinstance(dat, ContractReturn)
	return dat.data


def lazy_from_fd[T](
	fd: int, after: typing.Callable[[collections.abc.Buffer], T]
) -> Lazy[T]:
	def run():
		with os.fdopen(fd, 'rb') as f:
			return after(f.read())

	return Lazy(run)


class _LazyApi[T, **R]:
	def __init__(self, fn: typing.Callable[R, Lazy[T]]):
		self.fn = fn

	def __call__(self, *args: R.args, **kwargs: R.kwargs) -> T:
		"""
		Immediately execute and get the result
		"""
		return self.fn(*args, **kwargs).get()

	def lazy(self, *args: R.args, **kwargs: R.kwargs) -> Lazy[T]:
		"""
		Wrap evaluation into ``Lazy`` and return it
		"""
		return self.fn(*args, **kwargs)
