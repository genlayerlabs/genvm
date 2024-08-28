from .types import *
import abc

import typing

# FIXME
def hash_combine(l: Address, r: int) -> Address:
	hsh = hash((l, r))
	hsh *= 2
	if hsh < 0:
		hsh = 1 - hsh
	return Address(hsh.to_bytes(32))

class Storage(typing.Protocol):
	@abc.abstractmethod
	def get_store_slot(self, addr: Address) -> 'StorageSlot':
		pass

class StorageSlot:
	manager: Storage

	def __init__(self, addr: Address, manager: Storage):
		self.addr = addr
		self.manager = manager

	def indirect(self, off: int) -> 'StorageSlot':
		addr = hash_combine(self.addr, off)
		return self.manager.get_store_slot(addr)

	@abc.abstractmethod
	def read(self, addr: int, len: int) -> memoryview: ...

	@abc.abstractmethod
	def write(self, addr: int, what: memoryview) -> None: ...

class FakeStorage:
	mem = memoryview(bytearray(64))

class _ComplexCopyAction(typing.Protocol):
	@abc.abstractmethod
	def copy(self, frm: StorageSlot, frm_off: int, to: StorageSlot,to_off: int) -> int: ...

type _CopyAction = int | _ComplexCopyAction

class _TypeDesc[T]:
	size: int
	copy_actions: list[_CopyAction]

	def __init__(self, size: int, copy_atctions: list[_CopyAction]):
		self.copy_actions = copy_atctions
		self.size = size

	@abc.abstractmethod
	def get(self, slot: StorageSlot, off: int) -> T: ...

	@abc.abstractmethod
	def set(self, slot: StorageSlot, off: int, val: T) -> None: ...

class _IntDesc(_TypeDesc):
	def __init__(self, size: int, signed=True):
		_TypeDesc.__init__(self, size, [size])
		self.signed = signed

	def get(self, slot: StorageSlot, off: int) -> int:
		return int.from_bytes(slot.read(off, self.size), byteorder='little', signed=self.signed)

	def set(self, slot: StorageSlot, off: int, val: int):
		slot.write(off, memoryview(val.to_bytes(self.size, byteorder='little', signed=self.signed)))

class _AddrDesc(_TypeDesc):
	def __init__(self):
		_TypeDesc.__init__(self, Address.SIZE, [Address.SIZE])

	def get(self, slot: StorageSlot, off: int) -> Address:
		return Address(slot.read(off, self.size))

	def set(self, slot: StorageSlot, off: int, val: Address):
		slot.write(off, memoryview(val))

class _CopyStrBytesAction(_ComplexCopyAction):
	def __init__(self, size):
		self.size = size
	def copy(self, frm: StorageSlot, frm_off: int, to: StorageSlot, to_off: int) -> int:
		frm_stor = frm.indirect(frm_off)
		to_stor = to.indirect(to_off)
		u32_desc = _known_descs[u32]
		le = u32_desc.get(frm_stor, 0)

		# TODO: cow?
		to_stor.write(0, frm_stor.read(0, u32_desc.size + le))

		return self.size

class _StrBytesDescr[T](_TypeDesc):
	def __init__(self):
		size = 1
		_TypeDesc.__init__(self, size, [_CopyStrBytesAction(size)])

	@abc.abstractmethod
	def decode(self, val: memoryview) -> T: ...

	@abc.abstractmethod
	def encode(self, val: T) -> memoryview: ...

	def get(self, slot: StorageSlot, off: int) -> T:
		contents_at = slot.indirect(off)
		u32_desc = _known_descs[u32]
		le = u32_desc.get(contents_at, 0)
		return self.decode(contents_at.read(u32_desc.size, le))

	def set(self, slot: StorageSlot, off: int, val: T):
		contents_at = slot.indirect(off)
		enc = self.encode(val)
		u32_desc = _known_descs[u32]
		u32_desc.set(contents_at, 0, len(enc))
		contents_at.write(u32_desc.size, enc)

class _StrDescr(_StrBytesDescr):
	def __init__(self):
		_StrBytesDescr.__init__(self)

	@abc.abstractmethod
	def decode(self, val: memoryview) -> str:
		return str(val, encoding='utf-8')

	@abc.abstractmethod
	def encode(self, val: str) -> memoryview:
		return memoryview(val.encode())

class _BytesDescr(_StrBytesDescr):
	def __init__(self):
		_StrBytesDescr.__init__(self)

	@abc.abstractmethod
	def decode(self, val: memoryview) -> bytes:
		return bytes(val)

	@abc.abstractmethod
	def encode(self, val: bytes) -> memoryview:
		return memoryview(val)

_known_descs: dict[type, _TypeDesc] = {
	Address: _AddrDesc(),
	i64: _IntDesc(8),
	u32: _IntDesc(4, signed=False),
	str: _StrDescr(),
	bytes: _BytesDescr(),
}

class WithStorageSlot(typing.Protocol):
	__description__: _TypeDesc
	_storage_slot: StorageSlot
	_off: int

class _RecordDesc[T: WithStorageSlot](_TypeDesc):
	def __init__(self, view_ctor, size: int, actions: list[_CopyAction]):
		_TypeDesc.__init__(self, size, actions)
		self.view_ctor = view_ctor

	def get(self, slot: StorageSlot, off: int) -> T:
		return self.view_ctor(slot, off)

	def set(self, slot: StorageSlot, off: int, val: T) -> None:
		if self is not val.__description__:
			raise Exception("can't store")
		cum_off = 0
		from_stor = val._storage_slot
		from_off = val._off
		for act in self.copy_actions:
			if isinstance(act, int):
				slot.write(off + cum_off, from_stor.read(from_off + cum_off, act))
				cum_off += act
			else:
				cum_off += act.copy(from_stor, from_off + cum_off, slot, off + cum_off)

def _append_actions(l: list[_CopyAction], r: list[_CopyAction]):
	it = iter(r)
	if len(l) > 0 and len(r) > 0 and isinstance(l[-1], int) and isinstance(r[0], int):
		l[-1] += r[0]
		next(it)
	l.extend(it)

_default_extension = b"0" * 64

class _FakeStorageSlot(StorageSlot):
	_mem: bytearray
	def __init__(self, addr: Address, manager: Storage):
		StorageSlot.__init__(self, addr, manager)
		self._mem = bytearray()

	def read(self, addr: int, len: int) -> memoryview:
		return memoryview(self._mem[addr:addr+len])

	@abc.abstractmethod
	def write(self, addr: int, what: memoryview) -> None:
		l = len(what)
		while addr + l > len(self._mem):
			self._mem.extend(_default_extension)
		memoryview(self._mem)[addr:addr+l] = what

class _FakeStorageMan(Storage):
	_parts: dict[Address, _FakeStorageSlot] = {}

	def get_store_slot(self, addr: Address) -> StorageSlot:
		return self._parts.setdefault(addr, _FakeStorageSlot(addr, self))

	def debug(self):
		print("=== fake storage ===")
		for k, v in self._parts.items():
			print(f'{hex(int.from_bytes(k, byteorder="little"))}\n\t{v._mem}')

ROOT_STORAGE_ADDRESS = Address(bytes([0] * 32))

def storage(cls):
	size: int = 0
	copy_actions: list[_CopyAction] = []
	for prop_name, prop_value in typing.get_type_hints(cls).items():
		cur_offset = size
		desc = _known_descs[prop_value]
		def getter(s, desc=desc, cur_offset=cur_offset):
			return desc.get(s._storage_slot, s._off + cur_offset)
		def setter(s, v, desc=desc, cur_offset=cur_offset):
			desc.set(s._storage_slot, s._off + cur_offset, v)
		setattr(
			cls,
			prop_name,
			property(getter, setter)
		)
		size += desc.size
		_append_actions(copy_actions, desc.copy_actions)
	print(f'calculated size for {cls.__name__} is {size} actions are {copy_actions}')
	old_init = cls.__init__
	def patched_init(self, *args, **kwargs):
		if "gsdk_storage" in kwargs:
			storage = kwargs.pop("gsdk_storage")
			off = kwargs.pop("gsdk_offset")
		else:
			storage = _FakeStorageMan().get_store_slot(ROOT_STORAGE_ADDRESS)
			off = 0
		self._storage_slot = storage
		self._off = off
		old_init(self, *args, **kwargs)
	def view_at(slot: StorageSlot, off: int):
		slf = cls.__new__(cls)
		slf._storage_slot = slot
		slf._off = off
		return slf
	cls.__init__ = patched_init
	cls.view_at = staticmethod(view_at)
	description = _RecordDesc(view_at, size, copy_actions)
	_known_descs[cls] = description
	cls.__description__ = description
	return cls
