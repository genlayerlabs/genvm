__all__ = ('ContractAt', 'contract_interface')

import typing

from genlayer.py.types import Address, Lazy
import genlayer.py.calldata as calldata
import genlayer.std._wasi as wasi


from ._internal import decode_sub_vm_result, lazy_from_fd


def _make_calldata_obj(method, args, kwargs):
	ret = {'method': method}
	if len(args) > 0:
		ret.update({'args': args})
	if len(kwargs) > 0:
		ret.update({'kwargs': kwargs})
	return ret


class _ContractAtViewMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name

	def __call__(self, *args, **kwargs) -> typing.Any:
		return self.lazy(*args, **kwargs).get()

	def lazy(self, *args, **kwargs) -> Lazy[typing.Any]:
		obj = _make_calldata_obj(self.name, args, kwargs)
		cd = calldata.encode(obj)
		return lazy_from_fd(
			wasi.call_contract(self.addr.as_bytes, cd), decode_sub_vm_result
		)


class _ContractAtEmitMethod:
	def __init__(self, addr: Address, name: str, gas: int, code: bytes):
		self.addr = addr
		self.name = name
		self.gas = gas
		self.code = code

	def __call__(self, *args, **kwargs) -> None:
		obj = _make_calldata_obj(self.name, args, kwargs)
		cd = calldata.encode(obj)
		wasi.post_message(self.addr.as_bytes, cd, self.gas, self.code)


class _GenVMContract[TView, TSend](typing.Protocol):
	__slots__ = ('_view', '_send', 'address')

	def __init__(
		self,
		address: Address,
	):
		self.address = address

	def view(self) -> TView: ...

	def emit(self, *, gas: int) -> TSend: ...


class ContractAt(_GenVMContract):
	"""
	Provides a way to call view methods and send transactions to GenVM contracts
	"""

	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception('address expected')
		self.addr = addr

	def view(self):
		"""
		Namespace with all view methods
		"""
		return _ContractAtView(self.addr)

	def emit(self, *, gas: int, code: bytes = b''):
		"""
		Namespace with write message

		.. warning::
			parameters are subject to change!
		"""
		return _ContractAtEmit(self.addr, gas, code)


class _ContractAtView:
	def __init__(self, addr: Address):
		self.addr = addr

	def __getattr__(self, name):
		return _ContractAtViewMethod(self.addr, name)


class _ContractAtEmit:
	def __init__(self, addr: Address, gas: int, code: bytes):
		self.addr = addr
		self.gas = gas
		self.code = code

	def __getattr__(self, name):
		return _ContractAtEmitMethod(self.addr, name, self.gas, self.code)


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
) -> typing.Callable[[Address], _GenVMContract[TView, TWrite]]:
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
