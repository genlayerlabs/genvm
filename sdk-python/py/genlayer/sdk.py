import genlayer.wasi as wasi
import genlayer.calldata
import genlayer._storage as _storage
import collections.abc

import typing
import json
from types import SimpleNamespace
import base64

# reexports
from ._storage import storage, ROOT_STORAGE_ADDRESS
from ._storage_tree_map import TreeMap
from .types import *

def public(f):
	setattr(f, '__public__', True)
	return f

class AlreadySerializedResult(bytes):
	def __new__(cls, *args, **kwargs):
		return bytes.__new__(cls, *args, **kwargs)

def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)

class ContractMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name
	def __call__(self, *args):
		obj = {
			"method": self.name,
			"args": args,
		}
		calldata = genlayer.calldata.encode(obj)
		res = wasi.call_contract(self.addr.as_bytes, calldata)
		return genlayer.calldata.decode(res)

class Contract:
	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception("address expected")
		self.addr = addr
	def __getattr__(self, name):
		return ContractMethod(self.addr, name)


message_raw = json.loads(wasi.get_message_data())

message = SimpleNamespace(
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

def run_nondet(eq_principle, runner: Runner) -> typing.Any:
	import pickle
	res = wasi.run_nondet(json.dumps(eq_principle), pickle.dumps(runner))
	return genlayer.calldata.decode(res)

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
