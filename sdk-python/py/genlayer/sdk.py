import genlayer.wasi as wasi
import genlayer.py._storage as _storage
import collections.abc

import typing
import json
from types import SimpleNamespace as _SimpleNamespace
import base64
import os

# reexports
import genlayer.py.calldata as calldata
from .py._storage import storage, ROOT_STORAGE_ADDRESS
from .py._storage_tree_map import TreeMap
from .py.types import *

def public(f):
	setattr(f, '__public__', True)
	return f

class AwaitableResult:
	_exc: typing.Optional[Exception]
	_fd: int
	def __init__(self, fd: int):
		self._fd = fd
		self._exc = None
		self._res = None
	def __del__(self):
		if self._fd == 0:
			return
		os.close(self._fd)
		self._fd = 0
	def __await__(self):
		if self._fd == 0:
			if self._exc is not None:
				raise self._exc
			return self._res
		try:
			self._res = self._get_res(self._fd)
			return self._res
		except Exception as e:
			self._exc = e
			raise
		finally:
			self._fd = 0
		yield
	@abc.abstractmethod
	def _get_res(self, fd: int): ...

class AwaitableResultStr(AwaitableResult):
	def _get_res(self, fd: int) -> str:
		with os.fdopen(fd, "rt") as f:
			return f.read()

class AwaitableResultBytes(AwaitableResult):
	def _get_res(self, fd: int) -> bytes:
		with os.fdopen(fd, "rb") as f:
			return f.read()

class AwaitableResultBytesMap[T](AwaitableResult):
	def __init__(self, fd: int, fn: collections.abc.Callable[[bytes], T]):
		super().__init__(fd)
		self._fn = fn
	def _get_res(self, fd: int) -> T:
		with os.fdopen(fd, "rb") as f:
			return self._fn(f.read())

class AlreadySerializedResult(bytes):
	def __new__(cls, *args, **kwargs):
		return bytes.__new__(cls, *args, **kwargs)

def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)

class ContractMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name
	def __call__(self, *args) -> AwaitableResultBytesMap:
		obj = {
			"method": self.name,
			"args": args,
		}
		cd = calldata.encode(obj)
		res = wasi.call_contract(self.addr.as_bytes, cd)
		return AwaitableResultBytesMap(res, calldata.decode)

class Contract:
	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception("address expected")
		self.addr = addr
	def __getattr__(self, name):
		return ContractMethod(self.addr, name)


message_raw = json.loads(wasi.get_message_data())

message = _SimpleNamespace(
	gas=message_raw["gas"],
	contract_account=Address(message_raw["contract_account"]),
	sender_account=Address(message_raw["sender_account"]),
	value=message_raw.get("value", None),
	is_init=message_raw.get("is_init", None),
)

def rollback(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)

class Runner:
	def run(self):
		pass

def get_webpage(config: typing.Any, url: str) -> AwaitableResultStr:
	return AwaitableResultStr(wasi.get_webpage(json.dumps(config), url))

def call_llm(config: typing.Any, prompt: str) -> AwaitableResultStr:
	return AwaitableResultStr(wasi.call_llm(json.dumps(config), prompt))

def run_nondet(eq_principle, runner: Runner) -> AwaitableResultBytesMap:
	import pickle
	res = wasi.run_nondet(json.dumps(eq_principle), pickle.dumps(runner))
	return AwaitableResultBytesMap(res, calldata.decode)

class _ActualStorageMan(_storage.StorageMan):
	_slots: dict[Address, '_ActualStorageSlot']
	def __init__(self):
		self._slots = {}

	def get_store_slot(self, addr: Address) -> '_ActualStorageSlot':
		ret = self._slots.get(addr, None)
		if ret is None:
			ret = _ActualStorageSlot(addr, self)
			self._slots[addr] = ret
		return ret

class _ActualStorageSlot(_storage.StorageSlot):
	def __init__(self, addr: Address, manager: _storage.StorageMan):
		_storage.StorageSlot.__init__(self, addr, manager)

	def read(self, addr: int, len: int) -> bytes:
		return wasi.storage_read(self.addr.as_bytes, addr, len)

	@abc.abstractmethod
	def write(self, addr: int, what: collections.abc.Buffer) -> None:
		wasi.storage_write(self.addr.as_bytes, addr, what)

STORAGE_MAN = _ActualStorageMan()
