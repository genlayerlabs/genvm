"""
Module that is used to run python contracts in the default way
"""

__all__ = ()

import genlayer.std._wasi as wasi

entrypoint: bytes = wasi.get_entrypoint()
mem = memoryview(entrypoint)

import typing
import abc
from genlayer.py.types import Rollback
import genlayer.py.calldata
import genlayer.py._internal.reflect as reflect


def _give_result(res_fn: typing.Callable[[], typing.Any]):
	try:
		res = res_fn()
	except Rollback as r:
		wasi.rollback(r.msg)
	from ..advanced import AlreadySerializedResult

	if isinstance(res, AlreadySerializedResult):
		wasi.contract_return(res)
	else:
		wasi.contract_return(genlayer.py.calldata.encode(res))


SANDBOX = b'sandbox!'
if entrypoint.startswith(SANDBOX):
	import cloudpickle

	runner = cloudpickle.loads(mem[len(SANDBOX) :])
	_give_result(runner)

import contract as _user_contract_module  # type: ignore

import genlayer.std as gl
from ...py.storage._internal.generate import _known_descs

import genlayer.py.get_schema as _get_schema


class MethodIsNotPayable(Exception):
	"""
	method is not declared as payable
	"""


def _handle_call_special(contract: type, calldata: dict[str, typing.Any]) -> str:
	method_name = calldata.get('method', '')
	if method_name == '':
		return '__receive__'
	if method_name == '#error':
		return '__on_errored_message__'
	if method_name == '#get-schema':
		if get_schema := getattr(contract, '__get_schema__', None):
			if _get_schema._is_public(get_schema):
				raise TypeError('__get_schema__ must be private')
			_give_result(get_schema)

		from ...py.get_schema import get_schema
		import json

		_give_result(lambda: json.dumps(get_schema(contract), separators=(',', ':')))
	if method_name.startswith('#'):
		raise ValueError('method name can not start with hash sign')
	if method_name.startswith('__'):
		raise ValueError('method name can not start with two underscores')
	return method_name


def _handle_call(contract: type, mem: memoryview):
	calldata = genlayer.py.calldata.decode(mem)
	if not isinstance(calldata, dict):
		raise TypeError(
			f'invalid calldata, expected dict got `{reflect.repr_type(calldata)}`'
		)

	from .. import message

	is_undefined = False
	if message.is_init:
		meth = getattr(contract, '__init__')
		if _get_schema._is_public(meth):
			raise TypeError(f'constructor must be private')
		assert meth is not object.__init__
		meth_name = ''
	else:
		meth_name = _handle_call_special(contract, calldata)
		meth = getattr(contract, meth_name, None)
		if meth is None or getattr(meth, '__isabstractmethod__', False):
			is_undefined = True
			meth = getattr(contract, '__handle_undefined_method__')
		if meth is None or getattr(meth, '__isabstractmethod__', False):
			raise TypeError(
				'call to undefined method with absent __handle_undefined_method__ (fallback)'
			)
		if not _get_schema._is_public(meth):
			raise TypeError(f"can't call non-public methods")
		if message.value > 0 and not getattr(meth, _get_schema.PAYABLE_ATTR, False):
			raise TypeError('non-payable method called with value')

	from .storage import STORAGE_MAN, ROOT_STORAGE_ADDRESS

	top_slot = STORAGE_MAN.get_store_slot(ROOT_STORAGE_ADDRESS)
	contract_instance: gl.Contract = _known_descs[contract].get(top_slot, 0)
	if is_undefined:
		_give_result(
			lambda: contract_instance.__handle_undefined_method__(
				meth_name,
				calldata.get('args', []),  # type: ignore
				calldata.get('kwargs', {}),  # type: ignore
			)
		)
	else:
		_give_result(
			lambda: meth(
				contract_instance, *calldata.get('args', []), **calldata.get('kwargs', {})
			)
		)


def _handle_nondet(contract: type, mem: memoryview):
	# fetch leaders result length
	le = int.from_bytes(mem[:4], 'little')
	mem = mem[4:]

	leaders_res_mem = mem[:le]
	mem = mem[le:]

	import cloudpickle

	runner = cloudpickle.loads(mem)
	if le == 0:
		_give_result(runner)
	else:
		from . import decode_sub_vm_result_retn

		leaders_res = decode_sub_vm_result_retn(leaders_res_mem)
		_give_result(lambda: runner(leaders_res))


from ..genvm_contracts import __known_contact__

if __known_contact__ is None:
	raise Exception('no contract defined')

CALL = b'call!'
NONDET = b'nondet!'

if entrypoint.startswith(CALL):
	_handle_call(__known_contact__, mem[len(CALL) :])
elif entrypoint.startswith(NONDET):
	_handle_nondet(__known_contact__, mem[len(NONDET) :])
else:
	raise Exception(f'unknown entrypoint {entrypoint}')
