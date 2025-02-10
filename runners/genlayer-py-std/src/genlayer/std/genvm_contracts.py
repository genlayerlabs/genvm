__all__ = (
	'ContractAt',
	'contract_interface',
	'deploy_contract',
	'TransactionDataKwArgs',
	'DeploymentTransactionDataKwArgs',
	'Contract',
)

import typing
import json
import collections.abc

from genlayer.py.types import Address, Lazy, u256
import genlayer.py.calldata as calldata
import genlayer.std._wasi as wasi

from genlayer.py.eth.generate import transaction_data_kw_args_serialize

from ._internal import decode_sub_vm_result, lazy_from_fd_no_check


def _make_calldata_obj(method, args, kwargs):
	ret = {}
	if method is not None:
		ret['method'] = method
	if len(args) > 0:
		ret.update({'args': args})
	if len(kwargs) > 0:
		ret.update({'kwargs': kwargs})
	return ret


from ._internal.result_codes import StorageType


class GenVMCallKwArgs(typing.TypedDict):
	"""
	Built-in parameters of performing a GenVM call view method

	.. warning::
		parameters are subject to change!
	"""

	state: typing.NotRequired[StorageType]


class _ContractAtViewMethod:
	def __init__(self, addr: Address, name: str, data: GenVMCallKwArgs):
		self.addr = addr
		self.name = name
		data.setdefault('state', StorageType.DEFAULT)
		self.data = data

	def __call__(self, *args, **kwargs) -> typing.Any:
		return self.lazy(*args, **kwargs).get()

	def lazy(self, *args, **kwargs) -> Lazy[typing.Any]:
		obj = _make_calldata_obj(self.name, args, kwargs)
		cd = calldata.encode(obj)
		return lazy_from_fd_no_check(
			wasi.call_contract(self.addr.as_bytes, cd, json.dumps(self.data)),
			decode_sub_vm_result,
		)


class TransactionDataKwArgs(typing.TypedDict):
	"""
	Built-in parameters of all transaction messages that a contract can emit

	.. warning::
		parameters are subject to change!
	"""

	value: typing.NotRequired[u256]
	on: typing.NotRequired[typing.Literal['accepted', 'final']]


class _ContractAtEmitMethod:
	__slots__ = ('addr', 'name', 'data')

	def __init__(self, addr: Address, name: str | None, data: TransactionDataKwArgs):
		self.addr = addr
		self.name = name
		self.data = data

	def __call__(self, *args, **kwargs) -> None:
		obj = _make_calldata_obj(self.name, args, kwargs)
		cd = calldata.encode(obj)
		wasi.post_message(
			self.addr.as_bytes, cd, transaction_data_kw_args_serialize(dict(self.data))
		)


class GenVMContractProxy[TView, TSend](typing.Protocol):
	__slots__ = ('_view', '_send', 'address')

	address: Address
	"""
	Address to which this proxy points
	"""

	def __init__(
		self,
		address: Address,
	):
		self.address = address

	def view(self) -> TView: ...

	def emit(self, **kwargs: typing.Unpack[TransactionDataKwArgs]) -> TSend: ...

	@property
	def balance(self) -> u256: ...

	def emit_transfer(self, **data: typing.Unpack[TransactionDataKwArgs]): ...


class ContractAt(GenVMContractProxy):
	"""
	Provides a way to call view methods and send transactions to GenVM contracts
	"""

	__slots__ = ('address',)

	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception('address expected')
		self.address = addr

	def view(self):
		"""
		Namespace with all view methods

		:returns: object supporting ``.name(*args, **kwargs)`` that calls a contract and returns its result (:py:type:`~typing.Any`) or rises its :py:class:`~genlayer.py.types.Rollback`

		.. note::
			supports ``name.lazy(*args, **kwargs)`` call version
		"""
		return _ContractAtView(self.address, {})

	def emit(self, **data: typing.Unpack[TransactionDataKwArgs]):
		"""
		Namespace with write message

		:returns: object supporting ``.name(*args, **kwargs)`` that emits a message and returns :py:obj:`None`
		"""
		return _ContractAtEmit(self.address, data)

	def emit_transfer(self, **data: typing.Unpack[TransactionDataKwArgs]):
		"""
		Method to emit a message that transfers native tokens
		"""
		if 'value' not in data:
			raise TypeError('for emit_transfer value is required')
		_ContractAtEmitMethod(self.address, None, data)()

	@property
	def balance(self) -> u256:
		return u256(wasi.get_balance(self.address.as_bytes))


class _ContractAtView:
	__slots__ = ('addr', 'data')

	def __init__(self, addr: Address, data):
		self.addr = addr
		self.data = data

	def __getattr__(self, name):
		return _ContractAtViewMethod(self.addr, name, self.data)


class _ContractAtEmit:
	__slots__ = ('addr', 'data')

	def __init__(self, addr: Address, data: TransactionDataKwArgs):
		self.addr = addr
		self.data = data

	def __getattr__(self, name):
		return _ContractAtEmitMethod(self.addr, name, self.data)


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
) -> typing.Callable[[Address], GenVMContractProxy[TView, TWrite]]:
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
	return ContractAt


from genlayer.py.types import u8, u256


class DeploymentTransactionDataKwArgs(TransactionDataKwArgs):
	"""
	Class for representing parameters of ``deploy_contract``
	"""

	salt_nonce: typing.NotRequired[u256 | typing.Literal[0]]
	"""
	*iff* it is provided and does not equal to :math:`0` then ``Address`` of deployed contract will be known ahead of time. It will depend on this field
	"""


@typing.overload
def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[typing.Any] = [],
	kwargs: collections.abc.Mapping[str, typing.Any] = {},
) -> None: ...


@typing.overload
def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[typing.Any] = [],
	salt_nonce: typing.Literal[0],
	kwargs: collections.abc.Mapping[str, typing.Any] = {},
) -> Address: ...


def deploy_contract(
	*,
	code: bytes,
	args: collections.abc.Sequence[typing.Any] = [],
	kwargs: collections.abc.Mapping[str, typing.Any] = {},
	**data: typing.Unpack[DeploymentTransactionDataKwArgs],
) -> Address | None:
	"""
	Function for deploying new genvm contracts

	:param code: code (i.e. contents of a python file) of the contract

	:param args: arguments to be encoded into calldata
	:param kwargs: keyword arguments to be encoded into calldata

	:returns: address of new contract *iff* non-zero ``salt_nonce`` was provided

	.. info::
		Refer to consensus documentation for exact specification of

		- ``salt_nonce`` requirements and it's effect on address
		- order of transactions
	"""
	salt_nonce = data.setdefault('salt_nonce', u256(0))
	wasi.deploy_contract(
		calldata.encode(_make_calldata_obj(None, args, kwargs)),
		code,
		json.dumps(data),
	)
	if salt_nonce == 0:
		return None

	import genlayer.std as gl
	from genlayer.py._internal import create2_address

	return create2_address(gl.message.contract_account, salt_nonce, gl.message.chain_id)


import abc


class Contract:
	def __init_subclass__(cls) -> None:
		global __known_contact__
		if __known_contact__ is not None:
			raise TypeError(
				f'only one contract is allowed; first: `{__known_contact__}` second: `{cls}`'
			)

		cls.__contract__ = True
		from genlayer.py.storage._internal.generate import storage

		storage(cls)
		__known_contact__ = cls

	@property
	def balance(self) -> u256:
		return u256(wasi.get_self_balance())

	@abc.abstractmethod
	def __handle_undefined_method__(
		self, method_name: str, args: list[typing.Any], kwargs: dict[str, typing.Any]
	):
		"""
		Method that is called for no-method calls, must be either ``@gl.public.write`` or ``@gl.public.write.payable``
		"""
		...

	@abc.abstractmethod
	def __receive__(self):
		"""
		Method that is called for no-method transfers, must be ``@gl.public.write.payable``
		"""
		...


Contract.__handle_undefined_method__ = None  # type: ignore
Contract.__receive__ = None  # type: ignore

__known_contact__: type[Contract] | None = None
