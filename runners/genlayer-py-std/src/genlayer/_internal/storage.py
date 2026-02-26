__all__ = ('STORAGE_MAN', 'ROOT_SLOT_ID')

from ..storage._internal.core import Manager, Slot, ROOT_SLOT_ID
from ..types import u256

import _genlayer_wasi as wasi
import collections.abc


class _ActualStorageMan(Manager):
	__slots__ = ('_slots',)

	_slots: dict[bytes, Slot]

	def __init__(self):
		self._slots = {}

	def get_store_slot(self, addr: bytes) -> Slot:
		ret = self._slots.get(addr, None)
		if ret is None:
			ret = Slot(addr, self)
			self._slots[addr] = ret
		return ret

	def do_read(self, id: bytes, off: int, len: int) -> bytes:
		res = bytearray(len)
		wasi.storage_read(id, off, res)
		return bytes(res)

	def do_write(self, id: bytes, off: int, what: collections.abc.Buffer) -> None:
		wasi.storage_write(id, off, what)


STORAGE_MAN = _ActualStorageMan()
"""
Storage slots manager that provides an access to the "Host" (node) state
"""

from ..storage import Root

Root.MANAGER = STORAGE_MAN
