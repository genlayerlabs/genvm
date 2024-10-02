from genlayer.py.types import Address
import abc
import collections.abc

from .core import *

class _IntDesc(TypeDesc):
	def __init__(self, size: int, signed=True):
		TypeDesc.__init__(self, size, [size])
		self.signed = signed

	def get(self, slot: StorageSlot, off: int) -> int:
		return int.from_bytes(slot.read(off, self.size), byteorder='little', signed=self.signed)

	def set(self, slot: StorageSlot, off: int, val: int):
		slot.write(off, memoryview(val.to_bytes(self.size, byteorder='little', signed=self.signed)))

_u32_desc = _IntDesc(4, signed=False)

class _AddrDesc(TypeDesc):
	def __init__(self):
		TypeDesc.__init__(self, Address.SIZE, [Address.SIZE])

	def get(self, slot: StorageSlot, off: int) -> Address:
		return Address(slot.read(off, self.size))

	def set(self, slot: StorageSlot, off: int, val: Address):
		slot.write(off, memoryview(val.as_bytes))

class _CopyStrBytesAction(ComplexCopyAction):
	def __init__(self, size):
		self.size = size
	def copy(self, frm: StorageSlot, frm_off: int, to: StorageSlot, to_off: int) -> int:
		frm_stor = frm.indirect(frm_off)
		to_stor = to.indirect(to_off)
		le = _u32_desc.get(frm_stor, 0)

		to_stor.write(0, frm_stor.read(0, _u32_desc.size + le))

		return self.size

class _StrBytesDesc[T](TypeDesc):
	def __init__(self):
		size = 1
		TypeDesc.__init__(self, size, [_CopyStrBytesAction(size)])

	@abc.abstractmethod
	def decode(self, val: collections.abc.Buffer) -> T: ...

	@abc.abstractmethod
	def encode(self, val: T) -> memoryview: ...

	def get(self, slot: StorageSlot, off: int) -> T:
		contents_at = slot.indirect(off)
		le = _u32_desc.get(contents_at, 0)
		return self.decode(contents_at.read(_u32_desc.size, le))

	def set(self, slot: StorageSlot, off: int, val: T):
		contents_at = slot.indirect(off)
		enc = self.encode(val)
		_u32_desc.set(contents_at, 0, len(enc))
		contents_at.write(_u32_desc.size, enc)

class _StrDesc(_StrBytesDesc):
	def __init__(self):
		_StrBytesDesc.__init__(self)

	@abc.abstractmethod
	def decode(self, val: collections.abc.Buffer) -> str:
		return str(val, encoding='utf-8')

	@abc.abstractmethod
	def encode(self, val: str) -> memoryview:
		return memoryview(val.encode())

class _BytesDesc(_StrBytesDesc):
	def __init__(self):
		_StrBytesDesc.__init__(self)

	@abc.abstractmethod
	def decode(self, val: collections.abc.Buffer) -> bytes:
		return bytes(val)

	@abc.abstractmethod
	def encode(self, val: bytes) -> memoryview:
		return memoryview(val)
