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
	ctx = _BuilderCtx({}, ())
	desc = _storage_build(ctx, cls)

	_known_descs[cls] = desc
	return cls


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

_known_descs: dict[typing.Any, TypeDesc] = {
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


class _DeferredBuilder:
	__slots__ = ('_ctx', '_raw_type', '_cached', '_evaluated_at_ctx')

	def __init__(self, ctx: '_BuilderCtx', raw_type):
		self._ctx = ctx
		self._raw_type = raw_type
		self._cached: TypeDesc | None = None
		self._evaluated_at_ctx: '_BuilderCtx | None' = None

	@property
	def raw(self):
		return self._raw_type

	def get(self, instantiated_at_ctx: '_BuilderCtx') -> TypeDesc:
		if self._cached is None:
			self._evaluated_at_ctx = instantiated_at_ctx
			self._cached = _storage_build(self._ctx, self._raw_type)
		return self._cached


class GenerationError(TypeError):
	pass


class _BuilderCtx(typing.NamedTuple):
	generic_vars: dict[str, _DeferredBuilder]
	trace: tuple[str, ...]

	def with_trace(self, msg: str) -> '_BuilderCtx':
		new_val = self._replace(trace=self.trace + (msg,))
		return new_val

	def type_err(self, msg: str) -> GenerationError:
		exc = GenerationError(msg)
		for t in self.trace:
			exc.add_note(t)
		return exc

	@staticmethod
	def empty():
		return _BuilderCtx({}, ())


def _resolve_raw_type(ctx: _BuilderCtx, tp):
	"""Substitute TypeVars in a type using ctx.generic_vars, returning the raw type"""
	if isinstance(tp, typing.TypeVar):
		data = ctx.generic_vars.get(tp.__name__)
		if data is None:
			raise ctx.type_err(f'Unbound generic type variable `{tp.__name__}`')
		return _resolve_raw_type(ctx, data.raw)
	origin = typing.get_origin(tp)
	if origin is None:
		return tp
	args = typing.get_args(tp)
	new_args = tuple(_resolve_raw_type(ctx, a) for a in args)
	if new_args == args:
		return tp
	return origin[new_args] if len(new_args) > 1 else origin[new_args[0]]


def _storage_build(ctx: _BuilderCtx, cls) -> TypeDesc:
	if cached := _known_descs.get(cls):
		return cached

	trace_note = ['during building type `', reflect.repr_type(cls), '`']
	if len(ln := reflect.try_get_lineno(cls)) != 0:
		trace_note.append(' (declared at ')
		trace_note.append(str(ln))
		trace_note.append(')')

	ctx = ctx.with_trace(''.join(trace_note))
	return _storage_build_impl(ctx, cls)


def _check_forbidden_origin(ctx: _BuilderCtx, cls):
	if cls is int:
		raise ctx.type_err('use `bigint` or one of sized integers')
	if cls is dict:
		raise ctx.type_err('use `TreeMap` instead of a dict')
	if cls is list:
		raise ctx.type_err('use `DynArray` instead of a list')


def _storage_build_impl(ctx: _BuilderCtx, cls) -> TypeDesc:
	if s := _known_descs.get(cls):
		return s
	if isinstance(cls, typing.TypeVar):
		data = ctx.generic_vars.get(cls.__name__)
		if data is None:
			raise ctx.type_err(
				f'Unbound generic type variable `{cls.__name__}`, known variables: {",".join(ctx.generic_vars.keys())}'
			)
		try:
			return data.get(ctx)
		except BaseException as e:
			e.add_note(f'Instantiated generic variable `{cls.__name__}`')
			for t in ctx.trace:
				e.add_note(t)
			raise

	origin = typing.get_origin(cls)
	if origin is None:
		_check_forbidden_origin(ctx, cls)
		return _storage_build_struct(ctx._replace(generic_vars={}), cls)
	if 'numpy' in sys.modules and origin is sys.modules['numpy'].dtype:
		args = typing.get_args(cls)
		if len(args) != 1:
			raise ctx.type_err(
				f'Expected exactly one argument for numpy dtype, got {len(args)}'
			)
		return _storage_build(ctx, args[0])

	_check_forbidden_origin(ctx, origin)

	args = typing.get_args(cls)

	if origin is typing.Annotated:
		return _storage_build_annotated(ctx, cls)
	if origin is typing.Literal:
		raise ctx.type_err('Literal types are not supported in storage')
	if origin is tuple or origin is typing.Tuple:
		raise ctx.type_err(
			'Tuple types are not supported in storage, use a custom class instead'
		)

	if (as_np := try_handle_np(ctx, origin, args)) is not None:
		return as_np

	generic_params = origin.__type_params__

	if len(generic_params) != len(args):
		raise ctx.type_err(
			f'incorrect number of generic arguments for {origin} parameters={generic_params}, args={args}'
		)

	if origin is Array:
		return _storage_build_array(ctx, cls)
	if origin is DynArray:
		if len(typing.get_args(cls)) != 1:
			raise ctx.type_err(
				f'Expected exactly one argument for DynArray, got {len(typing.get_args(cls))}'
			)
		elem_type = typing.get_args(cls)[0]
		elem_desc = _storage_build(
			ctx.with_trace('during processing DynArray element type'), elem_type
		)
		return _DynArrayDesc(elem_desc)
	if origin is Indirection:
		if len(typing.get_args(cls)) != 1:
			raise ctx.type_err(
				f'Expected exactly one argument for Indirection, got {len(typing.get_args(cls))}'
			)
		elem_type = typing.get_args(cls)[0]
		elem_desc = _storage_build(
			ctx.with_trace('during processing Indirection element type'), elem_type
		)
		return IndirectionTypeDesc(elem_desc)
	if origin is VLA:
		if len(typing.get_args(cls)) != 1:
			raise ctx.type_err(
				f'Expected exactly one argument for VLA, got {len(typing.get_args(cls))}'
			)
		elem_type = typing.get_args(cls)[0]
		elem_desc = _storage_build(
			ctx.with_trace('during processing VLA element type'), elem_type
		)
		return VLATypeDesc(elem_desc)

	args = typing.get_args(cls)

	new_generics = {}
	for idx, (k, v) in enumerate(zip(generic_params, args)):
		trace_line = f'during processing generic argument of `{reflect.repr_generic(origin, args)}`, argument `{reflect.repr_type(v)}`, at index `{idx}`'
		var_ctx = ctx.with_trace(trace_line)
		new_generics[k.__name__] = _DeferredBuilder(var_ctx, v)
	ctx = ctx._replace(generic_vars=new_generics).with_trace(
		f'declared generic variables: {", ".join(new_generics.keys())}'
	)
	return _storage_build_struct(ctx, origin)


def _storage_build_array(ctx: _BuilderCtx, cls) -> TypeDesc:
	args = typing.get_args(cls)
	if len(args) != 2:
		raise ctx.type_err(f'Expected exactly two arguments for Array, got {len(args)}')
	type_arg, size_arg = args
	size_arg = _resolve_raw_type(ctx, size_arg)
	if typing.get_origin(size_arg) is not typing.Literal:
		raise ctx.type_err(f'Expected Literal for Array size, got {size_arg}')
	lit_args = typing.get_args(size_arg)
	if len(lit_args) != 1 or not isinstance(lit_args[0], int):
		raise ctx.type_err(f'Expected single int Literal for Array size, got {lit_args}')
	size = lit_args[0]
	child_type_desc = _storage_build(
		ctx.with_trace('during processing Array element type'), type_arg
	)
	res = _ArrayDesc(child_type_desc, size)
	return res


def _storage_build_annotated(ctx: _BuilderCtx, cls) -> TypeDesc:
	origin = getattr(cls, '__origin__', None)
	if origin is None:
		raise ctx.type_err('typing.Annotated should have __origin__')

	meta: tuple = getattr(cls, '__metadata__', ())
	if origin is int:
		for m in meta:
			if m == 'bigint':
				return _bigint_desc
			if isinstance(m, StaticIntMeta):
				return _int_descs[m][1]

	return _storage_build(ctx.with_trace('during processing discarded annotated'), origin)


def _storage_build_struct(ctx: _BuilderCtx, cls) -> TypeDesc:
	if not hasattr(cls, ALLOW_STORAGE_ATTR):
		raise ctx.type_err(
			f'class is not marked for usage within storage, please, annotate it with @allow_storage',
		)

	size: int = 0
	copy_actions: list[CopyAction] = []
	props: dict[str, tuple[TypeDesc, int]] = {}

	for prop_name, prop_value in typing.get_type_hints(cls, include_extras=True).items():
		if typing.get_origin(prop_value) is typing.ClassVar:
			continue

		cur_offset: int = size
		note = f'during processing field `{prop_name}: {prop_value}`'
		try:
			prop_desc = _storage_build(ctx.with_trace(note), prop_value)
			assert isinstance(prop_desc, TypeDesc)
		except GenerationError:
			raise
		except BaseException as e:
			e.add_note(note)
			raise
		props[prop_name] = (prop_desc, cur_offset)

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

		gen_var_name = None
		for generic_name, generic_info in ctx.generic_vars.items():
			if generic_info._evaluated_at_ctx is not None:
				gen_var_name = (generic_name, generic_info._evaluated_at_ctx)
				break

		if gen_var_name is not None:

			def new_init_generic(self, *args, **kwargs):
				if hasattr(self, '_storage_slot'):
					old_init(self, *args, **kwargs)
					return

				exc = gen_var_name[1].type_err(
					'generic storage classes can not be instantiated with __init__, please, use gl.storage.inmem_allocate'
				)
				exc.add_note(f'due to usage of `{gen_var_name[0]}`')
				exc.add_note(f'in class `{reflect.repr_type(cls)}`')
				raise exc

			new_init = new_init_generic
		else:

			def new_init_no_generic(self, *args, **kwargs):
				if not hasattr(self, '_storage_slot'):
					self._storage_slot = InmemManager().get_store_slot(ROOT_SLOT_ID)
					self._off = 0
					self.__type_desc__ = description
				old_init(self, *args, **kwargs)

			new_init = new_init_no_generic

		setattr(new_init, STORAGE_PATCHED_ATTR, True)
		setattr(new_init, ORIGINAL_INIT_ATTR, old_init)
		cls.__init__ = new_init
	return description


from .numpy import try_handle_np, populate_np_descs_if_loaded


@generate_storage
class _DateTime:
	seconds: u64
	micros: u32
	has_tz: bool
	off_days: i32
	off_seconds: i32
	off_micros: i32


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
