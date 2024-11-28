import genlayer._wasi as wasi

import typing
import json
from types import SimpleNamespace as _SimpleNamespace
import base64

# reexports
import genlayer.py.calldata as calldata
from ..py.types import *
from .eq_principles import *
from .nondet_fns import *

import genlayer.std.advanced as advanced

from ._private import decode_sub_vm_result, lazy_from_fd


def private(f):
	return f


class public:
	@staticmethod
	def view(f):
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', True)
		return f

	@staticmethod
	def write(f):
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', False)
		return f


def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)


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


class ContractAt:
	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception('address expected')
		self.addr = addr

	def view(self):
		return _ContractAtView(self.addr)

	def emit(self, *, gas: int, code: bytes = b''):
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


message_raw = json.loads(wasi.get_message_data())

message = _SimpleNamespace(
	contract_account=Address(message_raw['contract_account']),
	sender_account=Address(message_raw['sender_account']),
	value=message_raw.get('value', None),
	is_init=message_raw.get('is_init', None),
)


def rollback_immediate(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)


def contract(t: type) -> type:
	import inspect

	mod = inspect.getmodule(t)
	if mod is None:
		raise Exception(f"can't detect module where {t} is declared")
	if hasattr(mod, '__KNOWN_CONTRACT'):
		raise Exception(
			f'only one @contract is allowed, old {mod.__KNOWN_CONTRACT} new {t}'
		)
	t.__contract__ = True
	from genlayer.py.storage.generate import storage

	t = storage(t)
	setattr(mod, '__KNOWN_CONTRACT', t)
	return t
