import typing
import genlayer._wasi as wasi
import genlayer.py.calldata as calldata
from ..py.types import Rollback, Lazy
import collections.abc
import os


def _decode_sub_vm_result_retn(data: collections.abc.Buffer) -> typing.Any | Rollback:
	mem = memoryview(data)
	if mem[0] != 0:
		return Rollback(str(mem[1:], encoding='utf8'))
	return calldata.decode(mem[1:])


def _decode_sub_vm_result(data: collections.abc.Buffer) -> typing.Any:
	dat = _decode_sub_vm_result_retn(data)
	if isinstance(dat, Rollback):
		raise dat
	return dat


def _lazy_from_fd[T](
	fd: int, after: typing.Callable[[collections.abc.Buffer], T]
) -> Lazy[T]:
	def run():
		with os.fdopen(fd, 'rb') as f:
			return after(f.read())

	return Lazy(run)
