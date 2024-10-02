__all__ = ('STORAGE_MAN', 'ROOT_STORAGE_ADDRESS')

import genlayer.wasi as wasi
from genlayer.py.storage.core import *
from .py.storage import *
from .py.types import Address
import collections.abc
import abc

class _ActualStorageMan(StorageMan):
	_slots: dict[Address, '_ActualStorageSlot']
	def __init__(self):
		self._slots = {}

	def get_store_slot(self, addr: Address) -> '_ActualStorageSlot':
		ret = self._slots.get(addr, None)
		if ret is None:
			ret = _ActualStorageSlot(addr, self)
			self._slots[addr] = ret
		return ret

class _ActualStorageSlot(StorageSlot):
	def __init__(self, addr: Address, manager: StorageMan):
		super().__init__(addr, manager)

	def read(self, addr: int, len: int) -> bytes:
		return wasi.storage_read(self.addr.as_bytes, addr, len)

	@abc.abstractmethod
	def write(self, addr: int, what: collections.abc.Buffer) -> None:
		wasi.storage_write(self.addr.as_bytes, addr, what)

STORAGE_MAN = _ActualStorageMan()
