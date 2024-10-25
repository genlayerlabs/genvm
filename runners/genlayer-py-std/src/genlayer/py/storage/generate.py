__all__ = ('storage',)

from genlayer.py.types import *

import typing

from .core import *
from .core import _FakeStorageMan

from .desc_base_types import (
	_AddrDesc,
	_IntDesc,
	_StrDesc,
	_BytesDesc,
	_u32_desc,
	_BoolDesc,
)
from .desc_record import _RecordDesc, WithRecordStorageSlot
from .vec import DynArray, _DynArrayDesc, Array, _ArrayDesc


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
		if not isinstance(r, _Instantiation):
			return False
		return self.origin == r.origin and self.args == r.args

	def __hash__(self):
		return hash(('_Instantiation', self.origin, self.args))

	def __repr__(self):
		return f"{self.origin.__name__}[{', '.join(map(repr, self.args))}]"


_known_descs: dict[type | _Instantiation, TypeDesc] = {
	Address: _AddrDesc(),
	i8: _IntDesc(1),
	i64: _IntDesc(8),
	u32: _u32_desc,
	u64: _IntDesc(8, signed=False),
	u256: _IntDesc(32, signed=False),
	str: _StrDesc(),
	bytes: _BytesDesc(),
	bool: _BoolDesc(),
}


def _storage_build(
	cls: type | _Instantiation,
	generics_map: dict[str, TypeDesc],
) -> TypeDesc:
	if isinstance(cls, typing.TypeVar):
		return generics_map[cls.__name__]

	origin = typing.get_origin(cls)
	if origin is Array:
		args = typing.get_args(cls)
		assert len(args) == 2
		assert typing.get_origin(args[1]) is typing.Literal
		lit_args = typing.get_args(args[1])
		assert len(lit_args) == 1
		assert isinstance(lit_args[0], int)
		cls = _Instantiation(origin, (_storage_build(args[0], generics_map), lit_args[0]))  # type: ignore
	elif origin is not None:
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
	if cls.origin is DynArray:
		return _DynArrayDesc(cls.args[0])
	elif cls.origin is Array:
		return _ArrayDesc(cls.args[0], typing.cast(int, cls.args[1]))
	else:
		gen = {k.__name__: v for k, v in zip(generic_params, cls.args)}
		res = _storage_build_struct(cls.origin, gen)
		res.alias_to = cls
		return res


def _storage_build_struct(cls: type, generics_map: dict[str, TypeDesc]) -> TypeDesc:
	if cls is DynArray:
		raise Exception('invalid builder')
	size: int = 0
	copy_actions: list[CopyAction] = []
	props: dict[str, tuple[TypeDesc, int]] = {}

	was_generic = False

	for prop_name, prop_value in typing.get_type_hints(cls).items():
		cur_offset: int = size
		prop_desc = _storage_build(prop_value, generics_map)
		props[prop_name] = (prop_desc, cur_offset)

		if isinstance(prop_value, typing.TypeVar):
			was_generic = True

		if not getattr(cls, '__storage_patched__', False):

			def getter(s: WithRecordStorageSlot, prop_name=prop_name):
				prop_desc, off = s.__type_desc__.props[prop_name]
				return prop_desc.get(s._storage_slot, s._off + off)

			def setter(s: WithRecordStorageSlot, v, prop_name=prop_name):
				prop_desc, off = s.__type_desc__.props[prop_name]
				prop_desc.set(s._storage_slot, s._off + off, v)

			setattr(cls, prop_name, property(getter, setter))

		size += prop_desc.size
		actions_append(copy_actions, prop_desc.copy_actions)

	def view_at(desc: _RecordDesc, slot: StorageSlot, off: int, cls=cls):
		slf: WithRecordStorageSlot = cls.__new__(cls)  # type: ignore
		slf._storage_slot = slot
		slf._off = off
		slf.__type_desc__ = desc
		return slf

	description = _RecordDesc(view_at, size, copy_actions, props)

	old_init = cls.__init__

	def new_init(self, *args, **kwargs):
		if not hasattr(self, '_storage_slot'):
			assert not was_generic
			self._storage_slot = _FakeStorageMan().get_store_slot(ROOT_STORAGE_ADDRESS)
			self._off = 0
			self.__type_desc__ = description
		old_init(self, *args, **kwargs)

	new_init.__storage_patched__ = True

	if not hasattr(cls, '__contract__') and not getattr(
		old_init, '__storage_patched__', False
	):
		cls.__init__ = new_init
	return description
