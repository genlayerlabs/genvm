import typing

import genlayer.std._wasi as wasi
from ..py.types import Rollback, Lazy
import genlayer.py.calldata as calldata
from dataclasses import dataclass


class AlreadySerializedResult(bytes):
	def __new__(cls, *args, **kwargs):
		return bytes.__new__(cls, *args, **kwargs)


@dataclass
class ContractReturn:
	data: typing.Any


@dataclass
class ContractError:
	data: str


def run_nondet(
	leader_fn: typing.Callable[[], typing.Any],
	validator_fn: typing.Callable[[ContractReturn | Rollback | ContractError], bool],
) -> Lazy[typing.Any]:
	import cloudpickle
	from ._private import lazy_from_fd, decode_sub_vm_result

	fd = wasi.run_nondet(cloudpickle.dumps(leader_fn), cloudpickle.dumps(validator_fn))
	return lazy_from_fd(fd, decode_sub_vm_result)


def validator_handle_rollbacks_and_errors_default(
	fn: typing.Callable[[], typing.Any],
	leaders_result: ContractReturn | Rollback | ContractError,
) -> tuple[typing.Any, typing.Any]:
	try:
		res = fn()
		if not isinstance(leaders_result, ContractReturn):
			wasi.contract_return(calldata.encode(False))
		return (res, leaders_result.data)
	except Rollback as rb:
		wasi.contract_return(
			calldata.encode(
				isinstance(leaders_result, Rollback) and rb.msg == leaders_result.msg
			)
		)
	except Exception:
		wasi.contract_return(calldata.encode(isinstance(leaders_result, ContractError)))
