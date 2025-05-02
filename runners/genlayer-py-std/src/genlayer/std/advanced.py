"""
This module provides some "advanced" features that can be used for optimizations

.. warning::
	If you are using something "advanced" you must know what you do
"""

__all__ = (
	'ContractReturn',
	'ContractError',
	'run_nondet',
	'validator_handle_rollbacks_and_errors_default',
	'sandbox',
)

import typing

from ..py.types import Rollback, Lazy
import genlayer.py.calldata as calldata
from dataclasses import dataclass


@dataclass
class ContractReturn:
	"""
	Represents a normal "Return" result of a contract that is passed to validator function of :py:func:`genlayer.std.run_nondet`
	"""

	__slots__ = ('data',)

	data: calldata.Decoded


@dataclass
class ContractError(Exception):
	"""
	Represents "Contract error" result of a contract that is passed to validator function of :py:func:`genlayer.std.run_nondet`

	Validating leader output and sandbox invocation are only places where contract can "handle" contract error
	"""

	data: str


import genlayer.std._internal.gl_call as gl_call


def run_nondet[T: calldata.Decoded](
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

	return gl_call.gl_call_generic(
		{
			'RunNondet': {
				'data_leader': cloudpickle.dumps(leader_fn),
				'data_validator': cloudpickle.dumps(validator_fn),
			}
		},
		decode_sub_vm_result,
	)


def validator_handle_rollbacks_and_errors_default(
	fn: typing.Callable[[], calldata.Decoded],
	leaders_result: ContractReturn | Rollback | ContractError,
) -> tuple[calldata.Decoded, calldata.Decoded]:
	"""
	Default function to handle rollbacks and contract errors

	Errors and rollbacks are always checked for strict equality, which means that it's user responsibility to dump least possible text in there

	:returns: :py:class:`ContractReturn` data fields as ``(validator, leader)``` *iff* both results are not errors/rollbacks
	"""
	try:
		res = fn()
		if not isinstance(leaders_result, ContractReturn):
			gl_call.contract_return(False)
		return (res, leaders_result.data)
	except Rollback as rb:
		gl_call.contract_return(
			isinstance(leaders_result, Rollback) and rb.msg == leaders_result.msg
		)
	except Exception:
		gl_call.contract_return(isinstance(leaders_result, ContractError))


def sandbox[T: calldata.Decoded](fn: typing.Callable[[], T]) -> Lazy[T]:
	"""
	Runs function in the sandbox
	"""
	import cloudpickle
	from ._internal import lazy_from_fd_no_check, decode_sub_vm_result

	return gl_call.gl_call_generic(
		{
			'Sandbox': {
				'data': cloudpickle.dumps(fn),
			}
		},
		decode_sub_vm_result,
	)
