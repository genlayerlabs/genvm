__all__ = ('STORAGE_MAN', 'ROOT_STORAGE_ADDRESS')

import genlayer._wasi as wasi
from genlayer.py.storage.core import *
from .py.storage import *
from .py.types import Address
import collections.abc
import abc


class _ActualStorageMan(StorageMan):
	_slots: dict[u256, '_ActualStorageSlot']

	def __init__(self):
		self._slots = {}

	def get_store_slot(self, addr: u256) -> '_ActualStorageSlot':
		ret = self._slots.get(addr, None)
		if ret is None:
			ret = _ActualStorageSlot(addr, self)
			self._slots[addr] = ret
		return ret


class _ActualStorageSlot(StorageSlot):
	def __init__(self, addr: u256, manager: StorageMan):
		super().__init__(addr, manager)

	def read(self, addr: int, len: int) -> bytes:
		return wasi.storage_read(int.to_bytes(self.addr, 32, 'little'), addr, len)

	@abc.abstractmethod
	def write(self, addr: int, what: collections.abc.Buffer) -> None:
		wasi.storage_write(int.to_bytes(self.addr, 32, 'little'), addr, what)


STORAGE_MAN = _ActualStorageMan()
