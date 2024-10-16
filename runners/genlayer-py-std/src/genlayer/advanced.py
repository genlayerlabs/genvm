import typing

from ._private import _lazy_from_fd, _decode_sub_vm_result
import genlayer.wasi as wasi
from .py.types import Rollback, Lazy


class AlreadySerializedResult(bytes):
	def __new__(cls, *args, **kwargs):
		return bytes.__new__(cls, *args, **kwargs)


def run_nondet(
	leader_fn: typing.Callable[[], typing.Any],
	validator_fn: typing.Callable[[typing.Any | Rollback], bool],
) -> Lazy[typing.Any]:
	import cloudpickle

	fd = wasi.run_nondet(cloudpickle.dumps(leader_fn), cloudpickle.dumps(validator_fn))
	return _lazy_from_fd(fd, _decode_sub_vm_result)
