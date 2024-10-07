import typing
import genlayer.wasi as wasi
import genlayer.py.calldata as calldata
from .py.types import Rollback
from .asyn import AwaitableResultMap
import collections.abc


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


def _run_nondet(
	leader_fn: typing.Callable[[], typing.Any],
	validator_fn: typing.Callable[[typing.Any | Rollback], bool],
) -> AwaitableResultMap[typing.Any]:
	import cloudpickle

	fd = wasi.run_nondet(cloudpickle.dumps(leader_fn), cloudpickle.dumps(validator_fn))
	return AwaitableResultMap(fd, _decode_sub_vm_result)


def _call_user_fn(fn: typing.Callable[[], typing.Any]) -> typing.Any:
	res = fn()
	if hasattr(res, '__await__'):
		try:
			res.send(None)
		except StopIteration as si:
			return si.value
		raise Exception('invalid __await__ method')
	else:
		return res
