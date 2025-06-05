__all__ = (
	'spawn_sandbox',
	'run_nondet_unsafe',
	'run_nondet',
	'unpack_result',
	'Return',
	'VMError',
	'UserError',
	'Result',
)

import typing
import dataclasses
import collections.abc

from genlayer.py.types import Lazy
from ._internal import _lazy_api
import genlayer.py.calldata as calldata
import genlayer.gl._internal.gl_call as gl_call

from ._internal.result_codes import ResultCode


@dataclasses.dataclass
class Return[T: calldata.Decoded]:
	calldata: T


@dataclasses.dataclass
class VMError:
	message: str


@dataclasses.dataclass
class UserError(Exception):
	message: str


type Result[T: calldata.Decoded] = Return[T] | VMError | UserError


def _decode_sub_vm_result_retn(
	data: collections.abc.Buffer,
) -> Result:
	mem = memoryview(data)
	if mem[0] == ResultCode.ROLLBACK:
		return UserError(str(mem[1:], encoding='utf8'))
	if mem[0] == ResultCode.RETURN:
		return Return(calldata.decode(mem[1:]))
	if mem[0] == ResultCode.CONTRACT_ERROR:
		return VMError(str(mem[1:], encoding='utf8'))
	assert False, f'unknown type {mem[0]}'


def unpack_result[T: calldata.Decoded](res: Result[T], /) -> T:
	if isinstance(res, UserError):
		raise res
	if isinstance(res, VMError):
		raise UserError('vm error: ' + res.message)
	return res.calldata


def _decode_sub_vm_result(
	data: collections.abc.Buffer,
) -> calldata.Decoded:
	return unpack_result(_decode_sub_vm_result_retn(data))


@_lazy_api
def spawn_sandbox[T: calldata.Decoded](
	fn: typing.Callable[[], T], *, allow_write_ops: bool = False
) -> Lazy[Return[T] | VMError | UserError]:
	"""
	Runs function in the sandbox

	:param allow_write_ops: whether to allow write operations in the sandbox, has affect only if current VM has corresponding permission
	"""
	import cloudpickle
	from ._internal import decode_sub_vm_result

	return gl_call.gl_call_generic(
		{
			'Sandbox': {
				'data': cloudpickle.dumps(fn),
				'allow_write_ops': allow_write_ops,
			}
		},
		_decode_sub_vm_result_retn,
	)


@_lazy_api
def run_nondet_unsafe[T: calldata.Decoded](
	leader_fn: typing.Callable[[], T], validator_fn: typing.Callable[[Result], bool], /
) -> Lazy[T]:
	"""
	Most generic user-friendly api to execute a non-deterministic block

	:param leader_fn: function that is executed in the leader
	:param validator_fn: function that is executed in the validator that also checks leader result

	Uses :py:mod:`cloudpickle` to pass a "function" to sub VM

	.. note::
		This function does not use extra sandbox for catching validator errors.
		Validator error will result in a ``Disagree`` error in executor (same as if this function returned ``False``).
		You should use :py:func:`run_nondet` instead if you want to catch and inspect validator errors.

	.. warning::
		All sub-vm returns go through :py:mod:`genlayer.py.calldata` encoding
	"""

	import cloudpickle

	def validator_fn_mapped(stage_data):
		leaders_result = _decode_sub_vm_result_retn(stage_data['leaders_result'])
		return validator_fn(leaders_result)

	ret = gl_call.gl_call_generic(
		{
			'RunNondet': {
				'data_leader': cloudpickle.dumps(lambda _: leader_fn),
				'data_validator': cloudpickle.dumps(validator_fn_mapped),
			}
		},
		_decode_sub_vm_result,
	)

	return ret


def run_nondet[T: calldata.Decoded](
	leader_fn: typing.Callable[[], T],
	validator_fn: typing.Callable[[Result[T]], bool],
	/,
	*,
	compare_user_errors: typing.Callable[[UserError, UserError], bool] = lambda a,
	b: a.message == b.message,
	compare_vm_errors: typing.Callable[[VMError, VMError], bool] = lambda a, b: a.message
	== b.message,
) -> Lazy[T]:
	import cloudpickle

	def real_leader_fn(stage_data):
		assert stage_data is None
		return leader_fn()

	def real_validator_fn(stage_data) -> bool:
		leaders_result = _decode_sub_vm_result_retn(stage_data['leaders_result'])

		answer = spawn_sandbox(lambda: validator_fn(leaders_result), allow_write_ops=True)

		if type(answer) is not type(leaders_result):
			return False
		if isinstance(answer, Return):
			if not isinstance(answer, bool):
				raise TypeError(f'validator function returned non-bool `{answer}`')
			return answer.calldata
		elif isinstance(answer, UserError):
			return compare_user_errors(leaders_result, answer)

		return compare_vm_errors(leaders_result, answer)

	res = gl_call.gl_call_generic(
		{
			'RunNondet': {
				'data_leader': cloudpickle.dumps(real_leader_fn),
				'data_validator': cloudpickle.dumps(real_validator_fn),
			}
		},
		_decode_sub_vm_result,
	)

	return res
