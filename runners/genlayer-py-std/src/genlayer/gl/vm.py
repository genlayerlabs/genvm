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

from genlayer.py.public_abi import ResultCode


@dataclasses.dataclass
class Return[T: calldata.Decoded]:
	"""
	Represents a successful return value from a VM operation.
	"""

	calldata: T
	"""
	Decoded return value from the VM execution
	"""


@dataclasses.dataclass
class VMError:
	"""
	Represents an error that occurred within the VM during execution.

	It indicates user-caused error, such as OOM.
	"""

	message: str
	"""
	Description of the VM error that occurred. It begins with code, such as ``exit_code``
	"""


@dataclasses.dataclass
class UserError(Exception):
	"""
	Represents an error that user contract rose during execution of their code in the VM.
	"""

	message: str
	"""
	User-provided message. Be careful to use concise message, as by default they are checked for strict equality
	by the validator
	"""

	def __str__(self) -> str:
		return repr(self)


type Result[T: calldata.Decoded] = Return[T] | VMError | UserError
"""
Union type representing all possible outcomes from a VM operation.
"""


def _decode_sub_vm_result_retn(
	data: collections.abc.Buffer,
) -> Result:
	mem = memoryview(data)
	if mem[0] == ResultCode.USER_ERROR:
		return UserError(str(mem[1:], encoding='utf8'))
	if mem[0] == ResultCode.RETURN:
		return Return(calldata.decode(mem[1:]))
	if mem[0] == ResultCode.VM_ERROR:
		return VMError(str(mem[1:], encoding='utf8'))
	assert False, f'unknown type {mem[0]}'


def unpack_result[T: calldata.Decoded](res: Result[T], /) -> T:
	"""
	Extracts the successful result from a VM operation result.

	:param res: The result from a VM operation
	:return: The actual return value if successful
	:raises UserError: If the result represents a user error
	:raises UserError: If the result represents a ``VMError`` (rewrapped as user error)

	Example:
		>>> result = gl.vm.spawn_sandbox(lambda: 42)
		>>> value = unpack_result(result)  # Returns 42 or raises on error
	"""
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
	Runs a function in an isolated sandbox environment.

	The function is executed in a separate VM instance with controlled permissions.
	This provides isolation and security for potentially unsafe operations.
	Determinism of spawned VM matches the determinism of the current VM.

	:param fn: Function to execute in the sandbox (must be serializable with cloudpickle)
	:param allow_write_ops: Whether to allow write operations in the sandbox. Only effective if current VM has corresponding permission

	Example:
		>>> result = spawn_sandbox(lambda: risky_computation())
		>>> safe_value = unpack_result(result)
	"""
	import cloudpickle

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
	Executes a non-deterministic block with leader-validator consensus.

	This is the most generic API for non-deterministic execution. The leader function
	runs as is, validators one checks the result.

	:param leader_fn: Function executed by the leader node (must be serializable)
	:param validator_fn: Function that validates the leader's result and returns bool
	:return: The result from the leader (iff validation passes, otherwise VM will be terminated)

	.. warning::
		This function does not use extra sandbox for catching validator errors.
		Validator error will result in a ``Disagree`` error in executor (same as if
		this function returned ``False``). Use :py:func:`run_nondet` instead if you
		want to catch and inspect ``validator_fn`` errors, or use sandbox inside of it.

	.. note::
		All sub-vm returns go through :py:mod:`genlayer.py.calldata` encoding.

	Example:
		>>> def leader():
		...   return os.urandom(1)[0] % 3
		>>> def validator(result):
		...   return unpack_result(result) == 1  # agree in 33% of cases
		>>> value = gl.vm.run_nondet_unsafe(leader, validator)
	"""
	import cloudpickle

	def validator_fn_mapped(stage_data):
		leaders_result = _decode_sub_vm_result_retn(stage_data['leaders_result'])
		return validator_fn(leaders_result)

	ret = gl_call.gl_call_generic(
		{
			'RunNondet': {
				'data_leader': cloudpickle.dumps(lambda _: leader_fn()),
				'data_validator': cloudpickle.dumps(validator_fn_mapped),
			}
		},
		_decode_sub_vm_result,
	)

	return ret


@_lazy_api
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
	"""
	Executes a non-deterministic block with comprehensive error handling.

	This is the recommended API for custom non-deterministic execution. It provides safer
	error handling compared to :py:func:`run_nondet_unsafe` by running the validator
	in a sandbox and handling validator errors with provided functions with sensible defaults.

	:param leader_fn: Function executed by the leader node
	:param validator_fn: Function that validates the leader's result, is ran in a sandbox
	:param compare_user_errors: Function to compare UserError instances for equality
	:param compare_vm_errors: Function to compare VMError instances for equality
	:return: The result from the leader if validation passes

	Error handling:
	- If leader and validator both succeed: returns leader result
	- If leader fails and validator agrees: propagates leader error
	- If results don't match: consensus fails

	Example:
		>>> def leader() -> list[int]:
		...   return fetch_external_data()
		>>> def validator(result):
		...   if not isinstance(result, Return):
		...     return False
		...   my_data = leader()
		...   return numpy.linalg.norm(np.array(result.calldata) - np.array(my_data)) < 0.1
		>>> value = run_nondet(leader, validator)
	"""
	import cloudpickle

	def real_leader_fn(stage_data):
		assert stage_data is None
		return leader_fn()

	def real_validator_fn(stage_data) -> bool:
		leaders_result = _decode_sub_vm_result_retn(stage_data['leaders_result'])

		import genlayer.gl.vm as vm

		answer = vm.spawn_sandbox(
			lambda: validator_fn(leaders_result), allow_write_ops=True
		)

		if type(answer) is not type(leaders_result):
			return False
		if isinstance(answer, Return):
			if not isinstance(answer.calldata, bool):
				raise TypeError(f'validator function returned non-bool `{answer.calldata}`')
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
