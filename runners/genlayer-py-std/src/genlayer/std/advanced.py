"""
This module provides some "advanced" features that can be used for optimizations

.. warning::
	If you are using something "advanced" you must know what you do
"""

__all__ = (
	'AlreadySerializedResult',
	'ContractReturn',
	'ContractError',
	'run_nondet',
	'validator_handle_rollbacks_and_errors_default',
	'sandbox',
)

import typing

import genlayer.std._wasi as wasi
from ..py.types import Rollback, Lazy
import genlayer.py.calldata as calldata
from dataclasses import dataclass


class AlreadySerializedResult(bytes):
	"""
	If contract method returns instance of this class, calldata encoding won't be performed. Instead stored bytes will be passed as is
	"""

	__slots__ = ()

	def __new__(cls, *args, **kwargs):
		"""
		Forwards all arguments to :py:class:`bytes`
		"""
		return bytes.__new__(cls, *args, **kwargs)


@dataclass
class ContractReturn:
	"""
	Represents a normal "Return" result of a contract that is passed to validator function of :py:func:`genlayer.std.run_nondet`
	"""

	__slots__ = ('data',)

	data: typing.Any


@dataclass
class ContractError(Exception):
	"""
	Represents "Contract error" result of a contract that is passed to validator function of :py:func:`genlayer.std.run_nondet`

	Validating leader output and sandbox invocation are only places where contract can "handle" contract error
	"""

	data: str


def run_nondet[T](
	leader_fn: typing.Callable[[], T],
	validator_fn: typing.Callable[[ContractReturn | Rollback | ContractError], bool],
) -> Lazy[T]:
	"""
	Most generic user-friendly api to execute a non-deterministic block

	:param leader_fn: function that is executed in the leader
	:param validator_fn: function that is executed in the validator that also checks leader result

	Uses cloudpickle to pass a "function" to sub VM

	.. note::
		If validator_fn produces an error and leader_fn produces an error, executor itself will set result of this block to "agree" and fail entire contract with leader's error.
		This is done because not all errors can be caught in code itself (i.e. ``exit``).
		If this behavior is not desired, just fast return ``False`` for leader error result.

	.. warning::
		All sub-vm returns go through :py:mod:`genlayer.py.calldata` encoding
	"""
	import cloudpickle
	from ._internal import lazy_from_fd_no_check, decode_sub_vm_result

	fd = wasi.run_nondet(cloudpickle.dumps(leader_fn), cloudpickle.dumps(validator_fn))
	return lazy_from_fd_no_check(fd, decode_sub_vm_result)


def validator_handle_rollbacks_and_errors_default(
	fn: typing.Callable[[], typing.Any],
	leaders_result: ContractReturn | Rollback | ContractError,
) -> tuple[typing.Any, typing.Any]:
	"""
	Default function to handle rollbacks and contract errors

	Errors and rollbacks are always checked for strict equality, which means that it's user responsibility to dump least possible text in there

	:returns: :py:class:`ContractReturn` data fields as ``(validator, leader)``` *iff* both results are not errors/rollbacks
	"""
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


def sandbox(fn: typing.Callable[[], typing.Any]) -> Lazy[typing.Any]:
	"""
	Runs function in the sandbox
	"""
	import cloudpickle
	from ._internal import lazy_from_fd_no_check, decode_sub_vm_result

	return lazy_from_fd_no_check(
		wasi.sandbox(cloudpickle.dumps(fn)), decode_sub_vm_result
	)
