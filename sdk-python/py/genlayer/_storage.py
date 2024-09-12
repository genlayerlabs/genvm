from .types import *
import abc
import collections.abc

import typing
import types

# FIXME
def hash_combine(l: Address, r: int) -> Address:
	hsh = hash((l, r))
	hsh *= 2
	if hsh < 0:
		hsh = 1 - hsh
	btes = hsh.to_bytes(32)
	return Address(btes)

class StorageMan(typing.Protocol):
	@abc.abstractmethod
	def get_store_slot(self, addr: Address) -> 'StorageSlot':
		pass

class StorageSlot:
	manager: StorageMan

	def __init__(self, addr: Address, manager: StorageMan):
		self.addr = addr
		self.manager = manager

	def indirect(self, off: int) -> 'StorageSlot':
		addr = hash_combine(self.addr, off)
		return self.manager.get_store_slot(addr)

	@abc.abstractmethod
	def read(self, addr: int, len: int) -> bytes: ...

	@abc.abstractmethod
	def write(self, addr: int, what: collections.abc.Buffer) -> None: ...

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
		slot.write(off, memoryview(val.as_bytes))

_u32_desc = _IntDesc(4, signed=False)

class _CopyStrBytesAction(_ComplexCopyAction):
	def __init__(self, size):
		self.size = size
	def copy(self, frm: StorageSlot, frm_off: int, to: StorageSlot, to_off: int) -> int:
		frm_stor = frm.indirect(frm_off)
		to_stor = to.indirect(to_off)
		le = _u32_desc.get(frm_stor, 0)

		to_stor.write(0, frm_stor.read(0, _u32_desc.size + le))

		return self.size

class _StrBytesDescr[T](_TypeDesc):
	def __init__(self):
		size = 1
		_TypeDesc.__init__(self, size, [_CopyStrBytesAction(size)])

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

class _StrDescr(_StrBytesDescr):
	def __init__(self):
		_StrBytesDescr.__init__(self)

	@abc.abstractmethod
	def decode(self, val: collections.abc.Buffer) -> str:
		return str(val, encoding='utf-8')

	@abc.abstractmethod
	def encode(self, val: str) -> memoryview:
		return memoryview(val.encode())

class _BytesDescr(_StrBytesDescr):
	def __init__(self):
		_StrBytesDescr.__init__(self)

	@abc.abstractmethod
	def decode(self, val: collections.abc.Buffer) -> bytes:
		return bytes(val)

	@abc.abstractmethod
	def encode(self, val: bytes) -> memoryview:
		return memoryview(val)

class WithStorageSlot(typing.Protocol):
	__description__: _TypeDesc
	_storage_slot: StorageSlot
	_off: int

class WithItemStorageSlot(WithStorageSlot):
	_item_desc: _TypeDesc

class Vec[T](WithItemStorageSlot):
	__type_params__ = (typing.TypeVar('T'),)

	def __init__(self):
		raise Exception("this class can't be instantiated")

	def __class_getitem__(self, k):
		return typing._GenericAlias(self, (k,))

	@staticmethod
	def view_at(slot: StorageSlot, off: int) -> 'Vec':
		slf = Vec.__new__(Vec)
		slf._storage_slot = slot
		slf._off = off
		return slf

	def __len__(self) -> int:
		return _u32_desc.get(self._storage_slot, self._off)
	def _map_index(self, idx: int) -> int:
		le = len(self)
		if idx < 0:
			idx += le
		if idx < 0 or idx >= le:
			raise IndexError(f"index out of range {idx} not in 0..<{le}")
		return idx
	def __getitem__(self, idx: int) -> T:
		idx = self._map_index(idx)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.get(items_at, idx * self._item_desc.size)
	def __setitem__(self, idx: int, val: T) -> None:
		idx = self._map_index(idx)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.set(items_at, idx * self._item_desc.size, val)
	def __iter__(self) -> typing.Any:
		for i in range(len(self)):
			yield self[i]
	def append(self, item: T) -> None:
		le = len(self)
		_u32_desc.set(self._storage_slot, self._off, le + 1)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.set(items_at, le * self._item_desc.size, item)
	def append_new_get(self) -> T:
		le = len(self)
		_u32_desc.set(self._storage_slot, self._off, le + 1)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.get(items_at, le * self._item_desc.size)
	def pop(self) -> None:
		le = len(self)
		if le == 0:
			raise Exception("can't pop from empty array")
		_u32_desc.set(self._storage_slot, self._off, le - 1)

class _Instantiation:
	origin: type
	args: tuple[_TypeDesc, ...]
	def __init__(self, origin: type, args: tuple[_TypeDesc, ...]):
		self.origin = origin
		self.args = args
	def __eq__(self, r):
		if r is not _Instantiation:
			return False
		return self.origin == r.origin and self.args == r.args
	def __hash__(self):
		return hash((self.origin, self.args))

_known_descs: dict[type | _Instantiation, _TypeDesc] = {
	Address: _AddrDesc(),
	i8: _IntDesc(1),
	i64: _IntDesc(8),
	u32: _u32_desc,
	u64: _IntDesc(8, signed=False),
	str: _StrDescr(),
	bytes: _BytesDescr(),
}

def _apply_copy_actions(copy_actions: list[_CopyAction], to_stor: StorageSlot, to_off: int, frm_stor: StorageSlot, frm_off: int) -> int:
	cum_off = 0
	for act in copy_actions:
		if isinstance(act, int):
			to_stor.write(to_off + cum_off, frm_stor.read(frm_off + cum_off, act))
			cum_off += act
		else:
			cum_off += act.copy(frm_stor, frm_off + cum_off, to_stor, to_off + cum_off)
	return cum_off

class _RecordDesc[T: WithStorageSlot](_TypeDesc):
	def __init__(self, view_ctor, size: int, actions: list[_CopyAction]):
		_TypeDesc.__init__(self, size, actions)
		self.view_ctor = view_ctor

	def get(self, slot: StorageSlot, off: int) -> T:
		return self.view_ctor(slot, off)

	def set(self, slot: StorageSlot, off: int, val: T) -> None:
		if self is not val.__description__:
			raise Exception("can't store")
		_apply_copy_actions(self.copy_actions, slot, off, val._storage_slot, val._off)

def _append_actions(l: list[_CopyAction], r: list[_CopyAction]):
	it = iter(r)
	if len(l) > 0 and len(r) > 0 and isinstance(l[-1], int) and isinstance(r[0], int):
		l[-1] += r[0]
		next(it)
	l.extend(it)

_default_extension = b"\x00" * 64

class _FakeStorageSlot(StorageSlot):
	_mem: bytearray
	def __init__(self, addr: Address, manager: StorageMan):
		StorageSlot.__init__(self, addr, manager)
		self._mem = bytearray()

	def read(self, addr: int, len: int) -> bytes:
		return bytes(memoryview(self._mem)[addr:addr+len])

	def write(self, addr: int, what: memoryview) -> None:
		l = len(what)
		while addr + l > len(self._mem):
			self._mem.extend(_default_extension)
		memoryview(self._mem)[addr:addr+l] = what

class _FakeStorageMan(StorageMan):
	_parts: dict[Address, _FakeStorageSlot] = {}

	def get_store_slot(self, addr: Address) -> StorageSlot:
		return self._parts.setdefault(addr, _FakeStorageSlot(addr, self))

	def debug(self):
		print("=== fake storage ===")
		for k, v in self._parts.items():
			print(f'{hex(k.as_int)}\n\t{v._mem}')

ROOT_STORAGE_ADDRESS = Address(bytes([0] * 32))

def storage(cls):
	_storage_build(cls, {})
	return cls

class _VecCopyAction(_ComplexCopyAction):
	def __init__(self, item_desc: _TypeDesc):
		self._item_desc = item_desc

	def copy(self, frm: StorageSlot, frm_off: int, to: StorageSlot, to_off: int) -> int:
		le = _u32_desc.get(frm, frm_off)
		_u32_desc.set(to, to_off, le)
		cop = self._item_desc.copy_actions
		to_indir = to.indirect(to_off)
		frm_indir = frm.indirect(frm_off)
		if len(cop) == 1 and isinstance(cop[0], int):
			to_indir.write(0, frm_indir.read(0, self._item_desc.size * le))
		else:
			cum_off = 0
			for i in range(le):
				cum_off += _apply_copy_actions(cop, to, to_off + cum_off, frm, frm_off + cum_off)
		return _u32_desc.size

class _VecDescription(_TypeDesc):
	_item_desc: _TypeDesc
	def __init__(self, it_desc: _TypeDesc):
		self._item_desc = it_desc
		self._cop = _VecCopyAction(it_desc)
		_TypeDesc.__init__(self, _u32_desc.size, [self._cop])

	def get(self, slot: StorageSlot, off: int) -> Vec:
		res = Vec.view_at(slot, off)
		res._item_desc = self._item_desc
		return res

	def set(self, slot: StorageSlot, off: int, val: Vec) -> None:
		if val._item_desc is not self:
			raise Exception("incompatible vector type")
		self._cop.copy(val._storage_slot, val._off, slot, off)

def _storage_build(cls: type | _Instantiation, generics_map: dict[str, _TypeDesc]) -> _TypeDesc:
	if type(cls).__name__.endswith('TypeVar'):
		return generics_map[cls.__name__]
	if isinstance(cls, typing._GenericAlias):
		args = [_storage_build(c, generics_map) for c in cls.__args__]
		cls = _Instantiation(cls.__origin__, tuple(args))
	old = _known_descs.get(cls, None)
	if old is not None:
		return old
	if isinstance(cls, _Instantiation):
		description = _storage_build_generic(cls, generics_map)
	else:
		description = _storage_build_struct(cls, generics_map)
	_known_descs[cls] = description
	return description

def _storage_build_generic(cls: _Instantiation, generics_map: dict[str, _TypeDesc]) -> _TypeDesc:
	# here args are resolved but not instantiated
	tpars: tuple[typing.TypeVar, ...] = cls.origin.__type_params__
	if len(tpars) != len(cls.args):
		raise Exception(f"incorrect number of generic arguments parameters={tpars}, args={cls.args}")
	if cls.origin is Vec:
		return _VecDescription(cls.args[0])
	else:
		gen = {k.__name__: v for k, v in zip(tpars, cls.args)}
		return _storage_build_struct(cls.origin, gen)

def _storage_build_struct(cls: type, generics_map: dict[str, _TypeDesc]) -> _TypeDesc:
	if cls is Vec:
		raise Exception("invalid builder")
	size: int = 0
	copy_actions: list[_CopyAction] = []
	for prop_name, prop_value in typing.get_type_hints(cls).items():
		cur_offset = size
		desc = _storage_build(prop_value, generics_map)
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
	#import sys
	#print(f'calculated size for {cls.__name__}[{generics_map}] is {size} actions are {copy_actions}', file=sys.stderr)
	def view_at(slot: StorageSlot, off: int):
		slf = cls.__new__(cls)
		slf._storage_slot = slot
		slf._off = off
		return slf
	def view_at_root(man: StorageMan):
		return view_at( man.get_store_slot(ROOT_STORAGE_ADDRESS), 0)
	old_init = cls.__init__
	def new_init(self, *args, **kwargs):
		if not hasattr(self, '_storage_slot'):
			self._storage_slot = _FakeStorageMan().get_store_slot(ROOT_STORAGE_ADDRESS)
			self._off = 0
		old_init(self, *args, **kwargs)
	cls.__init__ = new_init
	cls.view_at = staticmethod(view_at)
	cls.view_at_root = staticmethod(view_at_root)
	description = _RecordDesc(view_at, size, copy_actions)
	cls.__description__ = description
	return description

from ._storage_tree_map import TreeMap
