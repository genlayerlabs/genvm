# NOTE: this file is needed to prevent numpy from loading into every contract

__all__ = ('try_handle_np',)

import sys
import typing

from .core import *

_imp: typing.Callable[..., TypeDesc | None] | None = None

_populated = False


def _populate_np_descs():
	global _populated
	if _populated:
		return
	_populated = True
	import numpy as np

	class _NumpyNDDesc(TypeDesc[np.ndarray]):
		__slots__ = ('shape', '_type')

		def __init__(self, typ: TypeDesc, shape: tuple[int, ...]):
			assert isinstance(typ, _NumpyDesc)
			type = typ._type
			dims = 1
			self.shape = shape
			for i in shape:
				dims *= i
			TypeDesc.__init__(self, type.itemsize * dims, [type.itemsize * dims])
			self._type = type

		def get(self, slot: Slot, off: int) -> np.ndarray:
			dat = slot.read(off, self.size)
			return np.frombuffer(dat, self._type).reshape(self.shape).copy()

		def set(self, slot: Slot, off: int, val: np.ndarray):
			assert val.dtype == self._type
			mv = memoryview(val).cast('B')
			assert len(mv) == self.size, f'invalid len {len(mv)} vs expected {self.size}'
			slot.write(off, mv)

	class _NumpyDesc(TypeDesc):
		__slots__ = ('_type', '_typ')

		def __init__(self, typ: np.number):
			type = np.dtype(typ)
			TypeDesc.__init__(self, type.itemsize, [type.itemsize])
			self._type = type
			self._typ = typ

		def get(self, slot: Slot, off: int):
			dat = slot.read(off, self.size)
			return np.frombuffer(dat, self._typ).reshape((1,))[0]

		def set(self, slot: Slot, off: int, val):
			slot.write(off, self._typ.tobytes(val))

	_all_np_types: list[type[np.number]] = [
		np.uint8,
		np.uint16,
		np.uint32,
		np.uint64,
		np.int8,
		np.int16,
		np.int32,
		np.int64,
		np.float32,
		np.float64,
	]
	_known_descs.update({k: _NumpyDesc(k) for k in _all_np_types})  # type: ignore

	def make_ndarray(ctx, origin, args) -> TypeDesc | None:
		if origin is not np.ndarray:
			return None

		if len(args) != 2:
			raise ctx.type_err(
				f'Expected exactly two arguments for np.ndarray, got {len(args)}'
			)

		shape_type = _resolve_raw_type(ctx, args[0])
		dtype_type = _resolve_raw_type(ctx, args[1])

		# parse shape: e.g. tuple[Literal[3], Literal[5]] → (3, 5)
		shape_origin = typing.get_origin(shape_type)
		if shape_origin is not tuple:
			raise ctx.type_err(f'Expected tuple for ndarray shape, got {shape_type}')

		shape_args = typing.get_args(shape_type)
		shape = []
		for dim in shape_args:
			if typing.get_origin(dim) is not typing.Literal:
				raise ctx.type_err(f'Expected Literal for ndarray dimension, got {dim}')
			lit_args = typing.get_args(dim)
			if len(lit_args) != 1 or not isinstance(lit_args[0], int):
				raise ctx.type_err(
					f'Expected single int Literal for ndarray dimension, got {lit_args}'
				)
			shape.append(lit_args[0])

		typ = _storage_build(
			ctx.with_trace('during processing ndarray element type'), dtype_type
		)
		return _NumpyNDDesc(typ, tuple(shape))

	global _imp
	_imp = make_ndarray


def try_handle_np(ctx, origin, args) -> TypeDesc | None:
	if _imp is None:
		return None
	return _imp(ctx, origin, args)


from .generate import _known_descs, _storage_build, _resolve_raw_type


def populate_np_descs_if_loaded():
	"""
	Call this function to populate numpy descs if numpy is loaded.
	"""
	if 'numpy' in sys.modules:
		_populate_np_descs()


populate_np_descs_if_loaded()
