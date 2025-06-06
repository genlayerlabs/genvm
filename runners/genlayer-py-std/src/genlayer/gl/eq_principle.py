__all__ = (
	'strict_eq',
	'prompt_comparative',
	'prompt_non_comparative',
)

import genlayer.gl.vm as vm
import typing
import genlayer.py.calldata as calldata

import genlayer.gl._internal.gl_call as gl_call

from ..py.types import *
from ._internal import (
	_lazy_api,
)


@_lazy_api
def strict_eq[T: calldata.Decoded](fn: typing.Callable[[], T]) -> Lazy[T]:
	"""
	Comparative equivalence principle that checks for strict equality

	This function checks that VM result is of the same type and has the same value inside.
	It is the most performant equivalence principle, but it is also the most strict one.

	:param fn: function that provides result that will be validated

	.. warning::
		See :py:func:`genlayer.gl.vm.run_nondet` for description of data transformations
	"""

	def validator_fn(
		leaders_res: vm.Result,
	) -> bool:
		my_res = vm.spawn_sandbox(fn)
		return my_res == leaders_res

	return vm.run_nondet_unsafe.lazy(fn, validator_fn)


from .nondet import _decode_nondet


@_lazy_api
def prompt_comparative[T: calldata.Decoded](
	fn: typing.Callable[[], T], principle: str
) -> Lazy[T]:
	"""
	Comparative equivalence principle that utilizes NLP for verifying that results are equivalent

	For validator: in case of non-``Return`` result in ``fn``,
	agreement will be decided by :py:func:`genlayer.gl.vm.run_nondet`,
	which executed validator wrapper function in a sandbox VM.
	If on the other hand leader reported an error, while our function execution is successful,
	the validator votes ``False``.

	:param fn: function that does all the job
	:param principle: principle with which equivalence will be evaluated in the validator (via performing NLP)

	See :py:func:`genlayer.gl.vm.run_nondet` for description of data transformations

	.. note::
		As leader results are encoded as calldata, :py:func:`format` is used for string representation. However, operating on strings by yourself is more safe in general

	.. warning::
		See :py:func:`genlayer.gl.vm.run_nondet` for description of data transformations
	"""

	def validator_fn(
		leaders_res: vm.Result,
	) -> bool:
		my_res = fn()

		if not isinstance(leaders_res, vm.Return):
			return False

		ret = gl_call.gl_call_generic(
			{
				'ExecPromptTemplate': {
					'template': 'EqComparative',
					'leader_answer': format(leaders_res.calldata),
					'validator_answer': format(my_res),
					'principle': principle,
				}
			},
			_decode_nondet,
		)

		return ret.get()

	return vm.run_nondet.lazy(fn, validator_fn)


@_lazy_api
def prompt_non_comparative(
	fn: typing.Callable[[], str], *, task: str, criteria: str
) -> Lazy[str]:
	"""
	Non-comparative equivalence principle that must cover most common use cases

	Both leader and validator finish their execution via NLP, that is used to perform ``task`` on ``input``.
	Leader just executes this task, but the validator checks if task was performed with integrity.
	This principle is useful when task is subjective. For instance, when you want to check if some text is a good summary of the input text.

	For validator: in case of non-``Return`` result in ``fn``,
	agreement will be decided by :py:func:`genlayer.gl.vm.run_nondet`,
	which executed validator wrapper function in a sandbox VM.
	If on the other hand leader reported an error, while our function execution is successful,
	the validator votes ``False``.
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
			_decode_nondet,
		)
		return ret.get()

	def validator_fn(
		leaders_res: vm.Result[str],
	) -> bool:
		my_res = fn()

		if not isinstance(leaders_res, vm.Return):
			return False

		ret = gl_call.gl_call_generic(
			{
				'ExecPromptTemplate': {
					'template': 'EqNonComparativeValidator',
					'task': task,
					'output': leaders_res.calldata,
					'input': my_res,
					'criteria': criteria,
				}
			},
			_decode_nondet,
		)
		ret = ret.get()
		return ret

	return vm.run_nondet.lazy(leader_fn, validator_fn)
