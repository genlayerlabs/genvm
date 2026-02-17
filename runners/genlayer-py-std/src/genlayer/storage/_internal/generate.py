"""
Module that uses reflections that generates python-friendly views to GenVM storage format (mapping from slot addresses to linear memories)
"""

__all__ = ('generate_storage',)

from genlayer.types import *
from genlayer.types import StaticIntMeta

import typing
import sys
import struct

from .core import *

from .desc_base_types import (
	AddrDesc,
	IntDesc,
	StrDesc,
	BytesDesc,
	BoolDesc,
	NoneDesc,
	_BigIntDesc,
)
from .desc_record import _RecordDesc, RecordExtraFields
from ..vec import DynArray, _DynArrayDesc, Array, _ArrayDesc

import genlayer._internal.reflect as reflect

STORAGE_PATCHED_ATTR = '__gl_storage_patched__'
ORIGINAL_INIT_ATTR = '__gl_original_init__'
ALLOW_STORAGE_ATTR = '__gl_allow_storage__'


def allow[T: type](cls: T) -> T:
	"""
	Marks class as allowed to be used within storage.
	Without this annotation, storage builder will raise an exception
	when trying to generate description for the class.
	This behavior is required to prevent accidental usage of classes that are not designed to be used in storage,
	because storage-generated class is modified and starts to behave differently from regular python class)
	"""
	setattr(cls, ALLOW_STORAGE_ATTR, True)
	return cls


def generate_storage[T: type](cls: T) -> T:
	populate_np_descs_if_loaded()
	cls = allow(cls)
	_storage_build(cls, {})
	return cls


class Lit:
	__slots__ = ()


class LitPy(Lit):
	__slots__ = ('alts',)

	def __init__(self, alts: tuple):
		self.alts = alts

	def __repr__(self):
		return f'LitPy[{" | ".join(repr(a) for a in self.alts)}]'


class LitTuple(Lit):
	__slots__ = ('args',)

	def __init__(self, args: tuple[Lit]):
		self.args = args

	def __repr__(self):
		return f'LitTuple[{" * ".join(repr(a) for a in self.args)}]'


class _Instantiation:
	origin: type
	args: tuple[TypeDesc | Lit, ...]

	__slots__ = ('origin', 'args')

	def __init__(self, origin: type, args: tuple[TypeDesc | Lit, ...]):
		self.origin = origin
		self.args = args

	def __eq__(self, r):
		if not isinstance(r, _Instantiation):
			return False
		return self.origin == r.origin and self.args == r.args

	def __hash__(self):
		return hash(('_Instantiation', self.origin, self.args))

	def __repr__(self):
		return f"{reflect.repr_type(self.origin)}[{', '.join(map(repr, self.args))}]"


_none_desc = NoneDesc()
_bigint_desc = _BigIntDesc()

_all_int_types: list = [
	u8,
	u16,
	u24,
	u32,
	u40,
	u48,
	u56,
	u64,
	u72,
	u80,
	u88,
	u96,
	u104,
	u112,
	u120,
	u128,
	u136,
	u144,
	u152,
	u160,
	u168,
	u176,
	u184,
	u192,
	u200,
	u208,
	u216,
	u224,
	u232,
	u240,
	u248,
	u256,
	i8,
	i16,
	i24,
	i32,
	i40,
	i48,
	i56,
	i64,
	i72,
	i80,
	i88,
	i96,
	i104,
	i112,
	i120,
	i128,
	i136,
	i144,
	i152,
	i160,
	i168,
	i176,
	i184,
	i192,
	i200,
	i208,
	i216,
	i224,
	i232,
	i240,
	i248,
	i256,
]

_int_descs: dict[StaticIntMeta, tuple[type, IntDesc]] = {}
for t in _all_int_types:
	sim: StaticIntMeta = t.__metadata__[0]
	_int_descs[sim] = (t, IntDesc(sim.size, signed=sim.signed))

_known_descs: dict[type | _Instantiation, TypeDesc] = {
	Address: AddrDesc(),
	str: StrDesc(),
	bytes: BytesDesc(),
	bool: BoolDesc(),
	type(None): _none_desc,
	None: _none_desc,  # type: ignore
	bigint: _bigint_desc,
}

for v in _int_descs.values():
	_known_descs[v[0]] = v[1]


class _FloatDesc(TypeDesc[float]):
	__slots__ = ('_type',)

	def __init__(self):
		TypeDesc.__init__(self, 8, [8])
		self._type = type

	def get(self, slot: Slot, off: int) -> float:
		dat = slot.read(off, self.size)
		return struct.unpack('d', dat)[0]

	def set(self, slot: Slot, off: int, val: float):
		slot.write(off, struct.pack('d', val))


_known_descs[float] = _FloatDesc()


def _storage_build_handle_special(
	origin: typing.Any,
	cls: type | _Instantiation,
	generics_map: dict[str, TypeDesc | Lit],
) -> tuple[bool, type | _Instantiation | Lit | TypeDesc]:
	if 'numpy' in sys.modules and origin is sys.modules['numpy'].dtype:
		args = typing.get_args(cls)
		assert len(args) == 1
		return True, _storage_build(args[0], generics_map)
	if origin is typing.Annotated:
		origin = getattr(cls, '__origin__', None)
		if origin is None:
			raise TypeError('typing.Annotated should have __origin__')

		meta: tuple = getattr(cls, '__metadata__', ())
		if origin is int:
			for m in meta:
				if m == 'bigint':
					return True, _bigint_desc
				if isinstance(m, StaticIntMeta):
					return True, _int_descs[m][1]

		with reflect.context_notes('during processing discarded annotated'):
			return True, _storage_build(origin, generics_map)
	if origin is typing.Literal:
		return True, LitPy(typing.get_args(cls))
	if origin is tuple:
		args = tuple(_storage_build(c, generics_map) for c in typing.get_args(cls))
		if all(isinstance(a, Lit) for a in args):
			return True, LitTuple(args)  # type: ignore
	if origin is Array:
		args = typing.get_args(cls)
		assert len(args) == 2
		assert typing.get_origin(args[1]) is typing.Literal
		lit_args = typing.get_args(args[1])
		assert len(lit_args) == 1
		assert isinstance(lit_args[0], int)
		res = _Instantiation(origin, (_storage_build(args[0], generics_map), lit_args[0]))  # type: ignore
		return True, res
	return False, cls


def _storage_build_inner(
	cls: type | _Instantiation,
	generics_map: dict[str, TypeDesc | Lit],
) -> TypeDesc | Lit:
	if cls is int:
		raise TypeError(
			'use `bigint` or one of sized integers please, see https://docs.genlayer.com/developers/intelligent-contracts/storage'
		)
	if isinstance(cls, typing.TypeVar):
		return generics_map[cls.__name__]

	origin = typing.get_origin(cls)
	special, new_cls_special = _storage_build_handle_special(origin, cls, generics_map)
	if special:
		if isinstance(new_cls_special, TypeDesc):
			return new_cls_special
		if isinstance(new_cls_special, Lit):
			return new_cls_special
		new_cls = new_cls_special
	elif origin is not None:
		args: list[TypeDesc | Lit] = []
		gen_args = typing.get_args(cls)
		for c_i, c in enumerate(gen_args):
			with reflect.context_generic_argument(origin, gen_args, c, c_i):
				args.append(_storage_build(c, generics_map))
		new_cls = _Instantiation(origin, tuple(args))
	else:
		new_cls = cls

	old = _known_descs.get(new_cls, None)
	if old is not None:
		return old
	if isinstance(new_cls, _Instantiation):
		description = _storage_build_generic(new_cls, generics_map)
	else:
		description = _storage_build_struct(new_cls, generics_map)
	_known_descs[new_cls] = description
	return description


from .numpy import try_handle_np, populate_np_descs_if_loaded


def _storage_build(
	cls: type | _Instantiation,
	generics_map: dict[str, TypeDesc | Lit],
) -> TypeDesc | Lit:
	with reflect.context_type(cls):
		return _storage_build_inner(cls, generics_map)


def _storage_build_generic(
	cls: _Instantiation, generics_map: dict[str, TypeDesc | Lit]
) -> TypeDesc:
	# here args are resolved but not instantiated
	generic_params = cls.origin.__type_params__

	assert cls.origin is not list, 'use DynArray'
	assert cls.origin is not dict, 'use TreeMap'

	if (as_np := try_handle_np(cls)) is not None:
		return as_np

	if len(generic_params) != len(cls.args):
		raise Exception(
			f'incorrect number of generic arguments for {cls.origin} parameters={generic_params}, args={cls.args}'
		)
	if cls.origin is DynArray:
		arg0 = cls.args[0]
		assert not isinstance(arg0, Lit)
		return _DynArrayDesc(arg0)
	elif cls.origin is Indirection:
		arg0 = cls.args[0]
		assert not isinstance(arg0, Lit)
		return IndirectionTypeDesc(arg0)
	elif cls.origin is VLA:
		arg0 = cls.args[0]
		assert not isinstance(arg0, Lit)
		return VLATypeDesc(arg0)
	elif cls.origin is Array:
		arg0 = cls.args[0]
		assert not isinstance(arg0, Lit)
		return _ArrayDesc(arg0, typing.cast(int, cls.args[1]))
	else:
		gen = {k.__name__: v for k, v in zip(generic_params, cls.args)}
		res = _storage_build_struct(cls.origin, gen)
		res.alias_to = cls
		return res


def _storage_build_struct(
	cls: type, generics_map: dict[str, TypeDesc | Lit]
) -> TypeDesc:
	if cls is DynArray:
		raise Exception('invalid builder')

	if not hasattr(cls, ALLOW_STORAGE_ATTR):
		raise TypeError(
			f'class is not marked for usage within storage, please, annotate it with @allow_storage',
			cls,
		)

	size: int = 0
	copy_actions: list[CopyAction] = []
	props: dict[str, tuple[TypeDesc, int]] = {}

	was_generic = False
	generic_info = {}

	for prop_name, prop_value in typing.get_type_hints(cls, include_extras=True).items():
		if typing.get_origin(prop_value) is typing.ClassVar:
			continue

		cur_offset: int = size
		try:
			prop_desc = _storage_build(prop_value, generics_map)
			assert isinstance(prop_desc, TypeDesc)
		except BaseException as e:
			e.add_note(f'during generating field `{prop_name}: {prop_value}`')
			raise
		props[prop_name] = (prop_desc, cur_offset)

		if isinstance(prop_value, typing.TypeVar):
			was_generic = True
			generic_info['name'] = prop_name
			generic_info['value'] = prop_value

		if not getattr(cls, STORAGE_PATCHED_ATTR, False):

			def getter(s: RecordExtraFields, prop_name=prop_name):
				prop_desc, off = s.__type_desc__.props[prop_name]
				return prop_desc.get(s._storage_slot, s._off + off)

			def setter(s: RecordExtraFields, v, prop_name=prop_name):
				prop_desc, off = s.__type_desc__.props[prop_name]
				prop_desc.set(s._storage_slot, s._off + off, v)

			setattr(cls, prop_name, property(getter, setter))

		size += prop_desc.size
		actions_append(copy_actions, prop_desc.copy_actions)

	description = _RecordDesc(size, copy_actions, props, cls)

	old_init = cls.__init__

	if not hasattr(cls, '__gl_contract__') and not getattr(
		old_init, STORAGE_PATCHED_ATTR, False
	):
		# here we may want to patch __init__ to allocate in storage
		def new_init_generic(self, *args, **kwargs):
			if hasattr(self, '_storage_slot'):
				old_init(self, *args, **kwargs)
				return

			exc = TypeError(
				'generic storage classes can not be instantiated with __init__, please, use gl.storage.inmem_allocate'
			)
			exc.add_note(
				f'due to field `{generic_info['name']}: {reflect.repr_type(generic_info['value'])}`'
			)
			exc.add_note(f'in class `{reflect.repr_type(cls)}`')
			raise exc

		def new_init_no_generic(self, *args, **kwargs):
			if not hasattr(self, '_storage_slot'):
				self._storage_slot = InmemManager().get_store_slot(ROOT_SLOT_ID)
				self._off = 0
				self.__type_desc__ = description
			old_init(self, *args, **kwargs)

		if was_generic:
			new_init = new_init_generic
		else:
			new_init = new_init_no_generic

		setattr(new_init, STORAGE_PATCHED_ATTR, True)
		setattr(new_init, ORIGINAL_INIT_ATTR, old_init)
		cls.__init__ = new_init
	return description


@generate_storage
class _DateTime:
	seconds: u64
	micros: u32
	has_tz: bool
	off_days: i32
	off_seconds: i32
	off_micros: i32


from functools import partial
import datetime, time

_dt_desc: TypeDesc[_DateTime] = _known_descs[_DateTime]


class _DateTimeDesc(TypeDesc[datetime.datetime]):
	__slots__ = ()

	def __init__(self):
		super().__init__(_dt_desc.size, _dt_desc.copy_actions)

	def get(self, slot: Slot, off: int) -> datetime.datetime:
		dt = _dt_desc.get(slot, off)

		def make_date(dt_tuple: time.struct_time, tzinfo):
			return datetime.datetime(
				year=dt_tuple.tm_year,
				month=dt_tuple.tm_mon,
				day=dt_tuple.tm_mday,
				hour=dt_tuple.tm_hour,
				minute=dt_tuple.tm_min,
				second=dt_tuple.tm_sec,
				microsecond=dt.micros,
				tzinfo=tzinfo,
			)

		if dt.has_tz:
			tz = datetime.timezone(
				datetime.timedelta(
					days=dt.off_days, seconds=dt.off_seconds, microseconds=dt.off_micros
				)
			)
			dt_tuple = time.gmtime(dt.seconds)
			return make_date(dt_tuple, datetime.UTC).astimezone(tz)
		else:
			tz = None
			dt_tuple = time.localtime(dt.seconds)
			return make_date(dt_tuple, tzinfo=tz)

	def set(self, slot: Slot, off: int, val: datetime.datetime) -> None:
		dt = _dt_desc.get(slot, off)
		tz = val.tzinfo
		dt.seconds = int(val.timestamp())
		dt.micros = val.microsecond
		if tz is None:
			dt.has_tz = False
		else:
			dt.has_tz = True
			tz_off = tz.utcoffset(None)
			assert tz_off is not None

			dt.off_days = tz_off.days
			dt.off_seconds = tz_off.seconds
			dt.off_micros = tz_off.microseconds


_known_descs[datetime.datetime] = _DateTimeDesc()

import genlayer.storage._internal.numpy
