__all__ = ('storage',)

from genlayer.py.types import *

import typing

from .core import *

from .desc_base_types import _AddrDesc, _IntDesc, _StrDesc, _BytesDesc, _u32_desc
from .desc_record import _RecordDesc
from .vec import Vec, _VecDescription


def storage(cls: type) -> type:
	_storage_build(cls, {})
	return cls


class _Instantiation:
	origin: type
	args: tuple[TypeDesc, ...]

	def __init__(self, origin: type, args: tuple[TypeDesc, ...]):
		self.origin = origin
		self.args = args

	def __eq__(self, r):
		if r is not _Instantiation:
			return False
		return self.origin == r.origin and self.args == r.args

	def __hash__(self):
		return hash((self.origin, self.args))


_known_descs: dict[type | _Instantiation, TypeDesc] = {
	Address: _AddrDesc(),
	i8: _IntDesc(1),
	i64: _IntDesc(8),
	u32: _u32_desc,
	u64: _IntDesc(8, signed=False),
	u256: _IntDesc(32, signed=False),
	str: _StrDesc(),
	bytes: _BytesDesc(),
}

_default_extension = b'\x00' * 64


class _FakeStorageSlot(StorageSlot):
	_mem: bytearray

	def __init__(self, addr: u256, manager: StorageMan):
		StorageSlot.__init__(self, addr, manager)
		self._mem = bytearray()

	def read(self, addr: int, len: int) -> bytes:
		return bytes(memoryview(self._mem)[addr : addr + len])

	def write(self, addr: int, what: memoryview) -> None:
		l = len(what)
		while addr + l > len(self._mem):
			self._mem.extend(_default_extension)
		memoryview(self._mem)[addr : addr + l] = what


class _FakeStorageMan(StorageMan):
	_parts: dict[u256, _FakeStorageSlot] = {}

	def get_store_slot(self, addr: u256) -> StorageSlot:
		return self._parts.setdefault(addr, _FakeStorageSlot(addr, self))

	def debug(self):
		print('=== fake storage ===')
		for k, v in self._parts.items():
			print(f'{hex(k)}\n\t{v._mem}')


def _storage_build(
	cls: type | _Instantiation, generics_map: dict[str, TypeDesc]
) -> TypeDesc:
	if isinstance(cls, typing.TypeVar):
		return generics_map[cls.__name__]
	origin = typing.get_origin(cls)
	if origin is not None:
		args = [_storage_build(c, generics_map) for c in typing.get_args(cls)]
		cls = _Instantiation(origin, tuple(args))
	old = _known_descs.get(cls, None)
	if old is not None:
		return old
	if isinstance(cls, _Instantiation):
		description = _storage_build_generic(cls, generics_map)
	else:
		description = _storage_build_struct(cls, generics_map)
	_known_descs[cls] = description
	return description


def _storage_build_generic(
	cls: _Instantiation, generics_map: dict[str, TypeDesc]
) -> TypeDesc:
	# here args are resolved but not instantiated
	generic_params = cls.origin.__type_params__
	if len(generic_params) != len(cls.args):
		raise Exception(
			f'incorrect number of generic arguments parameters={generic_params}, args={cls.args}'
		)
	if cls.origin is Vec:
		return _VecDescription(cls.args[0])
	else:
		gen = {k.__name__: v for k, v in zip(generic_params, cls.args)}
		return _storage_build_struct(cls.origin, gen)


def _storage_build_struct(cls: type, generics_map: dict[str, TypeDesc]) -> TypeDesc:
	if cls is Vec:
		raise Exception('invalid builder')
	cls_any: typing.Any = cls
	size: int = 0
	copy_actions: list[CopyAction] = []
	for prop_name, prop_value in typing.get_type_hints(cls).items():
		cur_offset = size
		desc = _storage_build(prop_value, generics_map)

		def getter(s, desc=desc, cur_offset=cur_offset):
			return desc.get(s._storage_slot, s._off + cur_offset)

		def setter(s, v, desc=desc, cur_offset=cur_offset):
			desc.set(s._storage_slot, s._off + cur_offset, v)

		setattr(cls, prop_name, property(getter, setter))
		size += desc.size
		actions_append(copy_actions, desc.copy_actions)

	def view_at(slot: StorageSlot, off: int):
		slf: typing.Any = cls.__new__(cls)  # type: ignore
		slf._storage_slot = slot
		slf._off = off
		return slf

	old_init = cls.__init__

	def new_init(self, *args, **kwargs):
		if not hasattr(self, '_storage_slot'):
			self._storage_slot = _FakeStorageMan().get_store_slot(ROOT_STORAGE_ADDRESS)
			self._off = 0
		old_init(self, *args, **kwargs)

	if not hasattr(cls, '__contract__'):
		cls.__init__ = new_init
	cls_any.__view_at__ = staticmethod(view_at)
	description = _RecordDesc(view_at, size, copy_actions)
	cls_any.__description__ = description
	return description
