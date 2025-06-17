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

type ON = typing.Literal['accepted', 'finalized']


class BaseContract(typing.Protocol):
	"""
	Base protocol for all GenVM contracts providing common properties.

	This protocol defines the minimal interface that all contract objects must implement,
	including access to balance and address information.
	"""

	@property
	def balance(self) -> u256:
		"""Current balance of the contract in native tokens."""
		...

	@property
	def address(self) -> Address:
		"""The contract's address on the blockchain."""
		...


def _make_calldata_obj(method, args, kwargs) -> calldata.Encodable:
	ret = {}
	if method is not None:
		ret['method'] = method
	if len(args) > 0:
		ret.update({'args': args})
	if len(kwargs) > 0:
		ret.update({'kwargs': kwargs})
	return ret


from genlayer.py.public_abi import StorageType


class _ContractAtViewMethod:
	__slots__ = ('_addr', '_name', '_state')

	def __init__(self, name: str, addr: Address, state: StorageType):
		self._addr = addr
		self._name = name
		self._state = state

	def __call__(self, *args, **kwargs) -> typing.Any:
		return self.lazy(*args, **kwargs).get()

	def lazy(self, *args, **kwargs) -> Lazy[typing.Any]:
		from genlayer.gl.vm import _decode_sub_vm_result

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
			_decode_sub_vm_result,
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
	"""
	Generic proxy interface for interacting with deployed GenVM contracts.

	This protocol defines the interface for contract proxies that provide type-safe
	access to view methods and write operations on deployed contracts.

	:param TView: Type representing available view methods
	:param TSend: Type representing available write methods
	"""

	def view(self, *, state: StorageType = StorageType.LATEST_NON_FINAL) -> TView:
		"""
		Get a namespace for calling view methods.

		:param state: Storage state to query against
		:returns: Object providing access to view methods
		"""
		...

	def emit(self, *, value: u256 = u256(0), on: ON = 'finalized') -> TSend:
		"""
		Get a namespace for emitting write transactions.

		:param value: Amount of native tokens to transfer with the transaction
		:param on: When the transaction message should be emitted to consensus
		:returns: Object providing access to write methods

		.. warning::
			Emitting transactions, especially with value transfers on ``accepted``
			may lead to undesired results. Prefer to use ``finalized`` (default)
		"""
		...

	def emit_transfer(self, *, value: u256, on: ON = 'finalized') -> None:
		"""
		Emit a simple value transfer without calling any method. Receiver may catch it with
		py:func:`genlayer.gl.Contract.__receive__` method, so users may need to supply non-zero gas

		:param value: Amount of native tokens to transfer
		:param on: When transaction message should be emitted to consensus

		:raises ValueError: If value is zero
		"""
		...


class ErasedMethods(typing.Protocol):
	"""
	Protocol for dynamically accessed contract methods.

	This protocol allows accessing contract methods by name when the exact
	interface is not known for type checker in IDE
	"""

	def __getattr__(self, name: str) -> typing.Callable:
		"""
		Get a callable for the named method.

		:param name: Name of the method to access
		:returns: Callable that can invoke the method
		"""
		...


class _ContractAt(ContractProxy[ErasedMethods, ErasedMethods]):
	__slots__ = ('_address',)

	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise TypeError('address expected')
		self._address = addr

	@property
	def address(self) -> Address:
		return self._address

	def view(self, *, state: StorageType = StorageType.LATEST_NON_FINAL) -> ErasedMethods:
		return _ContractAtGetter(_ContractAtViewMethod, self._address, state)

	def emit(self, *, value: u256 = u256(0), on: ON = 'finalized') -> ErasedMethods:
		return _ContractAtGetter(_ContractAtEmitMethod, self._address, value, on)

	def emit_transfer(self, *, value: u256, on: ON = 'finalized') -> None:
		if value <= 0:
			raise ValueError('value must be greater than 0 for emit_transfer')
		_ContractAtEmitMethod(None, self._address, value, on)()

	@property
	def balance(self) -> u256:
		return u256(wasi.get_balance(self._address.as_bytes))


def get_contract_at(address: Address) -> ContractProxy:
	"""
	Create a proxy object for interacting with a deployed GenVM contract.

	This function returns a contract proxy that provides runtime access to
	the methods of a deployed contract without requiring type annotations describing
	its interface.

	:param address: Address of the deployed contract
	:returns: ContractProxy object for interacting with the contract

	Example:
		>>> addr = Address('0x1234567890abcdef...')
		>>> contract = get_contract_at(addr)
		>>> result = contract.view().some_view_method(arg1, arg2)
		>>> contract.emit(value=u256(100)).some_write_method(arg1)
	"""
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
	"""
	Protocol for defining contract interface declarations.

	This protocol is used with the `@gl.contract_interface` decorator to create
	type-safe interfaces for interacting with specific contract types.

	:param TView: Type containing view method declarations
	:param TWrite: Type containing write method declarations

	Example:
		>>> @gl.contract_interface
		>>> class MyContract:
		>>>     class View:
		>>>         def get_balance(self, user: Address) -> u256: ...
		>>>         def get_name(self) -> str: ...
		>>>
		>>>     class Write:
		>>>         def transfer(self, to: Address, amount: u256) -> None: ...
		>>>         def mint(self, to: Address, amount: u256) -> None: ...
	"""

	View: type[TView]
	"""
	Class containing declarations for all view (read-only) methods.

	All methods should be annotated with their expected return types.
	"""

	Write: type[TWrite]
	"""
	Class containing declarations for all write (state-modifying) methods.

	All methods must have return type annotations of either None or be omitted.
	"""


def contract_interface[TView, TWrite](
	_declaration: GenVMContractDeclaration[TView, TWrite],
) -> typing.Callable[[Address], ContractProxy[TView, TWrite]]:
	# editorconfig-checker-disable
	"""
	Decorator for creating type-safe contract interfaces.

	This decorator creates a factory function that returns strongly-typed
	contract proxies, enabling IDE autocompletion and static type checking
	for contract interactions.

	:param _contr: Contract declaration class with View and Write nested classes
	:returns: Factory function that creates typed contract proxies

	Example:
		>>> @gl.contract_interface
		>>> class ERC20Contract:
		>>>     class View:
		>>>         def balance_of(self, owner: Address) -> u256: ...
		>>>         def total_supply(self) -> u256: ...
		>>>
		>>>     class Write:
		>>>         def transfer(self, to: Address, amount: u256) -> None: ...
		>>>         def approve(self, spender: Address, amount: u256) -> None: ...
		>>>
		>>> # Usage:
		>>> token = ERC20Contract(token_address)
		>>> balance = token.view().balance_of(user_address)  # Fully typed!
		>>> token.emit().transfer(recipient, amount)

	.. note::
		This decorator provides no runtime functionality - it's purely for
		type safety and developer experience. The actual contract interaction
		uses the same runtime mechanisms as `get_contract_at`.
	"""
	# editorconfig-checker-enable
	return get_contract_at


from genlayer.py.types import u8, u256


@typing.overload
def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[calldata.Encodable] = [],
	kwargs: collections.abc.Mapping[str, calldata.Encodable] = {},
	salt_nonce: typing.Literal[0] = 0,
	value: u256 = u256(0),
	on: ON = 'finalized',
) -> None: ...


@typing.overload
def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[calldata.Encodable] = [],
	kwargs: collections.abc.Mapping[str, calldata.Encodable] = {},
	salt_nonce: u256,
	value: u256,
	on: ON = 'finalized',
) -> Address: ...


def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[calldata.Encodable] = [],
	kwargs: collections.abc.Mapping[str, calldata.Encodable] = {},
	salt_nonce: u256 | typing.Literal[0] = u256(0),
	value: u256 = u256(0),
	on: ON = 'finalized',
) -> Address | None:
	"""
	Deploy a new GenVM contract to the blockchain.

	This function deploys a new contract using the provided ``code`` and
	constructor arguments. The deployment address can be deterministic (with a salt)
	or non-deterministic.

	:param code: Source code of the contract to deploy. It can be regular Python code. See :ref:`runners-reference` for more information
	:param args: Positional arguments for the contract constructor
	:param kwargs: Keyword arguments for the contract constructor
	:param salt_nonce: Salt for deterministic deployment. Use 0 for non-deterministic.
	:param value: Amount of native tokens to send to the contract during deployment
	:param on: When to execute the deployment ('accepted' or 'finalized')
	:returns: Contract address if salt_nonce != 0, None otherwise

	Example:
		>>> # Non-deterministic deployment
		>>> deploy_contract(
		>>>     code=contract_source_str.encode('utf-8'),
		>>>     args=[initial_supply],
		>>>     kwargs={"name": "MyToken", "symbol": "MTK"}
		>>> )
		>>>
		>>> # Deterministic deployment
		>>> address = deploy_contract(
		>>>     code=contract_source_zip_as_bytes,
		>>>     args=[initial_supply],
		>>>     salt_nonce=u256(12345),
		>>>     value=u256(1000)  # Send 1000 native tokens
		>>> )
		>>> print(f'Contract deployed at: {address}')

	.. note::
		- For deterministic deployments (salt_nonce != 0), the contract address
			is computed using CREATE2 and is returned immediately
		- For non-deterministic deployments (salt_nonce == 0), the address is
			assigned by the consensus and not returned. Considering asynchronous nature
			of GenLayer consensus the address should not be predicted
		- The contract's constructor will be called with the provided ``args`` and ``kwargs``
		- Refer to consensus documentation for CREATE2 address derivation process and
			details about transaction ordering
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


import genlayer.gl.annotations as glannots


class Contract(BaseContract):
	"""
	Class for declaring main GenVM contract.

	This class must be inherited by user contracts to be deployable on GenVM.
	It provides essential contract functionality including balance access,
	address information, and storage proxying.

	Only one ``Contract`` subclass is allowed per module. The class automatically
	generates storage management code and registers itself as the main contract.

	Example:
		>>> import genlayer.gl as gl
		>>>
		>>> class MyContract(gl.Contract):
		>>>     def __init__(self, initial_value: int):
		>>>         self.value = initial_value
		>>>
		>>>     @gl.public.view
		>>>     def get_value(self) -> int:
		>>>         return self.value
		>>>
		>>>     @gl.public.write
		>>>     def set_value(self, new_value: int):
		>>>         self.value = new_value

	.. warning::
		Only one Contract subclass is allowed per Python module. Attempting
		to define multiple Contract subclasses will raise a TypeError.
	"""

	def __init_subclass__(cls) -> None:
		"""
		Initialize the contract subclass and register it as the main contract.

		:raises TypeError: If another Contract subclass already exists in this module
		"""
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
		return u256(wasi.get_self_balance())

	@property
	def address(self) -> Address:
		from genlayer.gl import message

		return message.contract_address

	def __handle_undefined_method__(
		self, method_name: str, args: list[typing.Any], kwargs: dict[str, typing.Any]
	):
		"""
		Handle calls to undefined methods.

		This method is called when a message is sent to the contract with a method
		name that doesn't exist. If it is overriden, it must be either a ``gl.public.write``
		or ``gl.public.write.payable`` method.

		:param method_name: Name of the method that was called
		:param args: Positional arguments passed to the method
		:param kwargs: Keyword arguments passed to the method
		:raises NotImplementedError: Must be implemented by subclasses if used

		Example:
			>>> class MyContract(gl.Contract):
			>>>     @gl.public.write
			>>>     def __handle_undefined_method__(self, method_name: str, args: list, kwargs: dict):
			>>>         if method_name == "fallback_method":
			>>>             self.handle_fallback(args, kwargs)
			>>>         else:
			>>>             raise ValueError(f"Unknown method: {method_name}")
		"""
		raise NotImplementedError()

	def __receive__(self):
		"""
		Handle plain value transfers to this contract.

		This method is called when native tokens are sent to the contract
		without calling any specific method. It must be implemented as a
		public payable write method.

		:raises NotImplementedError: Must be implemented by subclasses if used

		Example:
			>>> class MyContract(gl.Contract):
			>>>     @gl.public.write.payable
			>>>     def __receive__(self):
			>>> # Handle incoming transfers
			>>>         sender = gl.message.sender
			>>>         value = gl.message.value
			>>>         self.total_received += value
		"""
		raise NotImplementedError()

	@glannots.public.write.payable
	def __on_errored_message__(self):
		"""
		Handle failed messages that included value transfers.

		This method is called when am emitted message with non-zero value fails during
		execution. By default, it simply accepts the refunded value. Override
		this method to implement custom error handling logic.

		The method must be decorated with ``@gl.public.write.payable``.

		Example:
			>>> class MyContract(gl.Contract):
			>>>     @gl.public.write.payable
			>>>     def __on_errored_message__(self):
			>>> # Log the failed transaction
			>>>         failed_value = gl.message.value
			>>>         self.failed_transfers.append((gl.message.sender, failed_value))
		"""
		pass

	@classmethod
	def __get_schema__(cls) -> str:
		"""
		Generate and return the JSON schema for this contract.

		This method analyzes the contract class and generates a JSON schema
		describing its public interface, including all public methods and
		their type signatures.

		:returns: JSON string containing the contract schema

		.. note::
			This method is used internally by the GenVM runtime for
			contract introspection and interface generation
		"""
		import genlayer.py.get_schema as _get_schema

		res = _get_schema.get_schema(cls)
		return json.dumps(res, separators=(',', ':'))


Contract.__handle_undefined_method__.__isabstractmethod__ = True  # type: ignore
Contract.__receive__.__isabstractmethod__ = True  # type: ignore

__known_contact__: type[Contract] | None = None
