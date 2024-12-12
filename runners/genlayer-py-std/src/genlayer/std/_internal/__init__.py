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


class LazyApi[T, **R](typing.Protocol):
	def __call__(self, *args: R.args, **kwargs: R.kwargs) -> T:
		"""
		Immediately execute and get the result
		"""
		...

	def lazy(self, *args: R.args, **kwargs: R.kwargs) -> Lazy[T]:
		"""
		Wrap evaluation into ``Lazy`` and return it
		"""
		...


def _lazy_api[T, **R](fn: typing.Callable[R, Lazy[T]]) -> LazyApi[T, R]:
	def eager(*args: R.args, **kwargs: R.kwargs) -> T:
		return fn(*args, **kwargs).get()

	if os.getenv('GENERATING_DOCS', 'false') == 'true':
		annots: dict = dict(fn.__annotations__)
		annots['return'] = annots['return'].__args__[0]
		eager.__annotations__ = annots
		import inspect
		import textwrap

		eager.__signature__ = inspect.signature(fn)
		eager.__doc__ = (
			textwrap.dedent(fn.__doc__)
			+ '\n\n.. note::\n\tsupports ``.lazy()`` version, which will return :py:class:`~genlayer.py.types.Lazy`'
		)
	eager.__name__ = fn.__name__
	eager.lazy = fn
	return eager
