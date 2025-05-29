__all__ = ('STORAGE_MAN', 'ROOT_SLOT_ID')

from ...py.storage._internal.core import *
from ...py.types import u256

import genlayer.std._wasi as wasi
import collections.abc
import abc


class _ActualStorageMan(Manager):
	__slots__ = ('_slots',)

	_slots: dict[bytes, '_ActualStorageSlot']

	def __init__(self):
		self._slots = {}

	def get_store_slot(self, addr: bytes) -> '_ActualStorageSlot':
		ret = self._slots.get(addr, None)
		if ret is None:
			ret = _ActualStorageSlot(addr, self)
			self._slots[addr] = ret
		return ret


class _ActualStorageSlot(Slot):
	__slots__ = ()

	def __init__(self, addr: bytes, manager: Manager):
		super().__init__(addr, manager)

	def read(self, addr: int, len: int) -> bytes:
		res = bytearray(len)
		wasi.storage_read(self.id, addr, res)
		return bytes(res)

	@abc.abstractmethod
	def write(self, addr: int, what: collections.abc.Buffer) -> None:
		wasi.storage_write(self.id, addr, what)


STORAGE_MAN = _ActualStorageMan()
"""
Storage slots manager that provides an access to the "Host" (node) state
"""
