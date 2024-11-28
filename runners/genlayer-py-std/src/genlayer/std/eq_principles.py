__all__ = (
	'eq_principle_strict_eq',
	'eq_principle_prompt_comparative',
	'eq_principle_prompt_non_comparative',
)

from .prompt_ids import *

import genlayer._wasi as wasi

import genlayer.std.advanced as advanced
import typing
import json
from ..py.types import *
from ._private import decode_sub_vm_result, lazy_from_fd, _LazyApi
from .nondet_fns import exec_prompt


def _eq_principle_strict_eq[T](fn: typing.Callable[[], T]) -> Lazy[T]:
	def validator_fn(
		leaders: advanced.ContractReturn | Rollback | advanced.ContractError,
	) -> bool:
		my_res, leaders_res = advanced.validator_handle_rollbacks_and_errors_default(
			fn, leaders
		)
		return my_res == leaders_res

	return advanced.run_nondet(fn, validator_fn)


eq_principle_strict_eq = _LazyApi(_eq_principle_strict_eq)
del _eq_principle_strict_eq


def _eq_principle_prompt_comparative(
	fn: typing.Callable[[], typing.Any], principle: str
) -> Lazy[str]:
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


eq_principle_prompt_comparative = _LazyApi(_eq_principle_prompt_comparative)
del _eq_principle_prompt_comparative


def _eq_principle_prompt_non_comparative(
	fn: typing.Callable[[], str], *, task: str, criteria: str
) -> Lazy[str]:
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


eq_principle_prompt_non_comparative = _LazyApi(_eq_principle_prompt_non_comparative)
del _eq_principle_prompt_non_comparative
