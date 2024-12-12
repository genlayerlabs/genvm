__all__ = (
	'eq_principle_strict_eq',
	'eq_principle_prompt_comparative',
	'eq_principle_prompt_non_comparative',
	'sandbox',
)

from ._internal.prompt_ids import *

import genlayer.std._wasi as wasi

import genlayer.std.advanced as advanced
import typing
import json
from ..py.types import *
from ._internal import decode_sub_vm_result_retn, lazy_from_fd, _lazy_api
from .nondet_fns import exec_prompt


@_lazy_api
def eq_principle_strict_eq[T](fn: typing.Callable[[], T]) -> Lazy[T]:
	"""
	Comparative equivalence principle that checks for strict equality

	:param fn: functions to perform an action
	"""

	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_res, leaders_res = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)
		return my_res == leaders_res

	return advanced.run_nondet(fn, validator_fn)


@_lazy_api
def eq_principle_prompt_comparative(
	fn: typing.Callable[[], typing.Any], principle: str
) -> Lazy[str]:
	"""
	Comparative equivalence principle that utilizes NLP for verifying that results are equivalent

	:param fn: function that does all the job
	:param principle: principle with which equivalence will be evaluated in the validator

	.. note::
		As leader results are encoded as calldata, :py:func:`format` is used for string representation. However, operating on strings by yourself is more safe in general
	"""

	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_res, leaders_res = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)
		vars = {
			'leader_answer': format(leaders_res),
			'validator_answer': format(my_res),
			'principle': principle,
		}
		return wasi.eq_principle_prompt(TemplateId.COMPARATIVE, json.dumps(vars))

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
	"""

	def leader_fn() -> str:
		input_res = fn()
		assert isinstance(input_res, str)
		return lazy_from_fd(
			wasi.exec_prompt_id(
				TemplateId.NON_COMPARATIVE_LEADER,
				json.dumps(
					{
						'task': task,
						'input': input_res,
						'criteria': criteria,
					}
				),
			),
			lambda buf: str(buf, 'utf-8'),
		).get()

	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_input, leaders_result = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)
		vars = {
			'task': task,
			'output': leaders_result,
			'input': my_input,
			'criteria': criteria,
		}
		return wasi.eq_principle_prompt(TemplateId.NON_COMPARATIVE, json.dumps(vars))

	return advanced.run_nondet(leader_fn, validator_fn)


@_lazy_api
def sandbox(data: typing.Callable[[], typing.Any]) -> Lazy[typing.Any]:
	"""
	Runs function in the sandbox

	.. warning::
		It returns result via pickle, which can be unsafe. If it is not desired wrap it to bytes yourself
	"""
	import cloudpickle
	import genlayer.py.calldata as calldata

	def decode(x: collections.abc.Buffer):
		res = decode_sub_vm_result_retn(x)
		if isinstance(res, advanced.ContractReturn):
			return cloudpickle.loads(res.data)
		if isinstance(res, Rollback):
			raise res
		if isinstance(res, advanced.ContractError):
			raise Exception(res)

	return lazy_from_fd(wasi.sandbox(cloudpickle.dumps(data)), decode)
