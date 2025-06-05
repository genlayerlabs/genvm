__all__ = (
	'contract_interface',
	'deploy_contract',
	'Contract',
	'get_contract_at',
	'BaseContract',
	'ContractProxy',
)

import typing
import json
import collections.abc

from genlayer.py.types import Address, Lazy, u256
import genlayer.py.calldata as calldata

import _genlayer_wasi as wasi

from ._internal.gl_call import gl_call_generic
from ._internal import decode_sub_vm_result

type ON = typing.Literal['accepted', 'finalized']


class BaseContract(typing.Protocol):
	@property
	def balance(self) -> u256: ...
	@property
	def address(self) -> Address: ...


def _make_calldata_obj(method, args, kwargs) -> calldata.Encodable:
	ret = {}
	if method is not None:
		ret['method'] = method
	if len(args) > 0:
		ret.update({'args': args})
	if len(kwargs) > 0:
		ret.update({'kwargs': kwargs})
	return ret


from ._internal.result_codes import StorageType


class _ContractAtViewMethod:
	__slots__ = ('_addr', '_name', '_state')

	def __init__(self, name: str, addr: Address, state: StorageType):
		self._addr = addr
		self._name = name
		self._state = state

	def __call__(self, *args, **kwargs) -> typing.Any:
		return self.lazy(*args, **kwargs).get()

	def lazy(self, *args, **kwargs) -> Lazy[typing.Any]:
		obj = _make_calldata_obj(self._name, args, kwargs)
		cd = calldata.encode(obj)
		return gl_call_generic(
			{
				'CallContract': {
					'address': self._addr,
					'calldata': _make_calldata_obj(self._name, args, kwargs),
					'state': self._state.value,
				}
			},
			decode_sub_vm_result,
		)


class _ContractAtEmitMethod:
	__slots__ = ('_addr', '_name', '_value', '_on')

	def __init__(self, name: str | None, addr: Address, value: u256, on: str):
		self._addr = addr
		self._name = name
		self._value = value
		self._on = on

	def __call__(self, *args, **kwargs) -> None:
		wasi.gl_call(
			calldata.encode(
				{
					'PostMessage': {
						'address': self._addr,
						'calldata': _make_calldata_obj(self._name, args, kwargs),
						'value': self._value,
						'on': self._on,
					}
				}
			)
		)


class ContractProxy[TView, TSend](BaseContract, typing.Protocol):
	def view(self, *, state: StorageType = StorageType.LATEST_NON_FINAL) -> TView: ...
	def emit(self, *, value: u256 = u256(0), on: ON = 'finalized') -> TSend: ...
	def emit_transfer(self, *, value: u256, on: ON = 'finalized') -> None: ...


class ErasedMethods(typing.Protocol):
	def __getattr__(self, name: str) -> typing.Callable: ...


class _ContractAt(ContractProxy[ErasedMethods, ErasedMethods]):
	"""
	Provides a way to call view methods and send transactions to GenVM contracts
	"""

	__slots__ = ('_address',)

	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise TypeError('address expected')
		self._address = addr

	@property
	def address(self) -> Address:
		return self._address

	def view(self, *, state: StorageType = StorageType.LATEST_NON_FINAL) -> ErasedMethods:
		"""
		Namespace with all view methods

		:returns: object supporting ``.name(*args, **kwargs)`` that calls a contract and returns its result (:py:type:`~typing.Any`) or rises its :py:class:`~genlayer.py.types.Rollback`

		.. note::
			supports ``name.lazy(*args, **kwargs)`` call version
		"""
		return _ContractAtGetter(_ContractAtViewMethod, self._address, state)

	def emit(self, *, value: u256 = u256(0), on: ON = 'finalized') -> ErasedMethods:
		"""
		Namespace with write message

		:returns: object supporting ``.name(*args, **kwargs)`` that emits a message and returns :py:obj:`None`
		"""
		return _ContractAtGetter(_ContractAtEmitMethod, self._address, value, on)

	def emit_transfer(self, *, value: u256, on: ON = 'finalized') -> None:
		"""
		Method to emit a message that transfers native tokens
		"""
		_ContractAtEmitMethod(None, self._address, value, on)()

	@property
	def balance(self) -> u256:
		return u256(wasi.get_balance(self._address.as_bytes))


def get_contract_at(address: Address) -> ContractProxy:
	return _ContractAt(address)


_ContractAtGetter_P = typing.ParamSpec('_ContractAtGetter_P')


class _ContractAtGetter[T]:
	__slots__ = ('_ctor', '_args', '_kwargs')

	def __init__(
		self,
		ctor: typing.Callable[typing.Concatenate[str, _ContractAtGetter_P], T],
		*args: _ContractAtGetter_P.args,
		**kwargs: _ContractAtGetter_P.kwargs,
	):
		self._ctor = ctor
		self._args = args
		self._kwargs = kwargs

	def __getattr__(self, name: str) -> T:
		return self._ctor(name, *self._args, **self._kwargs)


class GenVMContractDeclaration[TView, TWrite](typing.Protocol):
	View: type[TView]
	"""
	Class that contains declarations for all view methods
	"""
	Write: type[TWrite]
	"""
	Class that contains declarations for all write methods

	.. note::
		all return type annotations must be either empty or ``None``
	"""


def contract_interface[TView, TWrite](
	_contr: GenVMContractDeclaration[TView, TWrite],
) -> typing.Callable[[Address], ContractProxy[TView, TWrite]]:
	# editorconfig-checker-disable
	"""
	This decorator produces an "interface" for other GenVM contracts. It has no semantical value, but can be used for auto completion and type checks

	.. code-block:: python

	        @gl.contract_interface
	        class MyContract:
	          class View:
	            def view_meth(self, i: int) -> int: ...

	          class Write:
	            def write_meth(self, i: int) -> None: ...
	"""
	# editorconfig-checker-enable
	return get_contract_at


from genlayer.py.types import u8, u256


@typing.overload
def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[typing.Any] = [],
	kwargs: collections.abc.Mapping[str, typing.Any] = {},
	salt_nonce: typing.Literal[0],
	value: u256 = u256(0),
	on: ON = 'finalized',
) -> None: ...


@typing.overload
def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[typing.Any] = [],
	kwargs: collections.abc.Mapping[str, typing.Any] = {},
	salt_nonce: u256,
	value: u256 = u256(0),
	on: ON = 'finalized',
) -> Address: ...


def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[typing.Any] = [],
	kwargs: collections.abc.Mapping[str, typing.Any] = {},
	salt_nonce: u256 | typing.Literal[0] = u256(0),
	value: u256 = u256(0),
	on: ON = 'finalized',
) -> Address | None:
	"""
	Function for deploying new genvm contracts

	:param code: code (i.e. contents of a python file) of the contract

	:param args: arguments to be encoded into calldata
	:param kwargs: keyword arguments to be encoded into calldata

	:returns: address of new contract *iff* non-zero ``salt_nonce`` was provided

	.. note::
		Refer to consensus documentation for exact specification of

		- ``salt_nonce`` requirements and it's effect on address
		- order of transactions
	"""

	wasi.gl_call(
		calldata.encode(
			{
				'DeployContract': {
					'calldata': _make_calldata_obj(None, args, kwargs),
					'code': code,
					'value': value,
					'on': on,
					'salt_nonce': salt_nonce,
				}
			}
		)
	)

	if salt_nonce == 0:
		return None

	import genlayer.gl as gl
	from genlayer.py._internal import create2_address

	return create2_address(gl.message.contract_address, salt_nonce, gl.message.chain_id)


import abc

import genlayer.gl.annotations as glannots


class Contract(BaseContract):
	"""
	Class that indicates main user contract
	"""

	def __init_subclass__(cls) -> None:
		global __known_contact__
		if __known_contact__ is not None:
			raise TypeError(
				f'only one contract is allowed; first: `{__known_contact__}` second: `{cls}`'
			)

		cls.__gl_contract__ = True
		from genlayer.py.storage._internal.generate import generate_storage

		generate_storage(cls)
		__known_contact__ = cls

	@property
	def balance(self) -> u256:
		"""
		Current balance of this contract
		"""
		return u256(wasi.get_self_balance())

	@property
	def address(self) -> Address:
		"""
		:returns: :py:class:`Address` of this contract
		"""
		from genlayer.gl import message

		return message.contract_address

	def __handle_undefined_method__(
		self, method_name: str, args: list[typing.Any], kwargs: dict[str, typing.Any]
	):
		"""
		Method that is called for no-method calls, must be either ``@gl.public.write`` or ``@gl.public.write.payable``
		"""
		raise NotImplementedError()

	def __receive__(self):
		"""
		Method that is called for no-method transfers, must be ``@gl.public.write.payable``
		"""
		raise NotImplementedError()

	@glannots.public.write.payable
	def __on_errored_message__(self):
		"""
		Method that is called when emitted message with non-zero value failed. This method is not abstract to just receive value.
		It must be ``@gl.public.write.payable``
		"""
		pass

	@classmethod
	def __get_schema__(cls) -> str:
		import genlayer.py.get_schema as _get_schema

		res = _get_schema.get_schema(cls)
		return json.dumps(res, separators=(',', ':'))


Contract.__handle_undefined_method__.__isabstractmethod__ = True  # type: ignore
Contract.__receive__.__isabstractmethod__ = True  # type: ignore

__known_contact__: type[Contract] | None = None
