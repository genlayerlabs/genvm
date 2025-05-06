__all__ = (
	'eq_principle_strict_eq',
	'eq_principle_prompt_comparative',
	'eq_principle_prompt_non_comparative',
)

import genlayer.std._wasi as wasi

import genlayer.std.advanced as advanced
import typing
import json
import genlayer.py.calldata as calldata

import genlayer.std._internal.gl_call as gl_call

from ..py.types import *
from ._internal import (
	_lazy_api,
)


@_lazy_api
def eq_principle_strict_eq[T: calldata.Decoded](fn: typing.Callable[[], T]) -> Lazy[T]:
	"""
	Comparative equivalence principle that checks for strict equality

	:param fn: functions to perform an action

	See :py:func:`genlayer.std.advanced.run_nondet` for description of data transformations
	"""

	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_res, leaders_res = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)
		return my_res == leaders_res

	return advanced.run_nondet(fn, validator_fn)


from .nondet_fns import _decode_nondet


@_lazy_api
def eq_principle_prompt_comparative[T: calldata.Decoded](
	fn: typing.Callable[[], T], principle: str
) -> Lazy[T]:
	"""
	Comparative equivalence principle that utilizes NLP for verifying that results are equivalent

	:param fn: function that does all the job
	:param principle: principle with which equivalence will be evaluated in the validator (via performing NLP)

	See :py:func:`genlayer.std.advanced.run_nondet` for description of data transformations

	.. note::
		As leader results are encoded as calldata, :py:func:`format` is used for string representation. However, operating on strings by yourself is more safe in general
	"""

	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_res, leaders_res = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)
		ret = gl_call.gl_call_generic(
			{
				'ExecPromptTemplate': {
					'template': 'EqComparative',
					'leader_answer': format(leaders_res),
					'validator_answer': format(my_res),
					'principle': principle,
				}
			},
			_decode_nondet,
		)

		return ret.get()

	return advanced.run_nondet(fn, validator_fn)


@_lazy_api
def eq_principle_prompt_non_comparative(
	fn: typing.Callable[[], str], *, task: str, criteria: str
) -> Lazy[str]:
	"""
	Non-comparative equivalence principle that must cover most common use cases

	Both leader and validator finish their execution via NLP, that is used to perform ``task`` on ``input``.
	Leader just executes this task, but the validator checks if task was performed with integrity.
	This principle is useful when task is subjective

	See :py:func:`~genlayer.std.advanced.run_nondet` for description of data transformations
	"""

	def leader_fn() -> str:
		input_res = fn()
		assert isinstance(input_res, str)

		ret = gl_call.gl_call_generic(
			{
				'ExecPromptTemplate': {
					'template': 'EqNonComparativeLeader',
					'task': task,
					'input': input_res,
					'criteria': criteria,
				}
			},
			lambda buf: str(buf, 'utf-8'),
		)
		return ret.get()

	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_input, leaders_result = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)

		ret = gl_call.gl_call_generic(
			{
				'ExecPromptTemplate': {
					'template': 'EqNonComparativeValidator',
					'task': task,
					'output': leaders_result,
					'input': my_input,
					'criteria': criteria,
				}
			},
			_decode_nondet,
		)
		ret = ret.get()
		return ret

	return advanced.run_nondet(leader_fn, validator_fn)
