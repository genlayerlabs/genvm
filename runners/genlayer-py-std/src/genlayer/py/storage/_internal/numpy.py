# NOTE: this file is needed to prevent numpy from loading into every contract

__all__ = ('try_handle_np',)

import sys
import typing

from .core import *

_imp: typing.Callable[['_Instantiation'], TypeDesc | None] | None = None


def _populate_np_descs():
	import numpy as np

	class _NumpyNDDesc(TypeDesc[np.ndarray]):
		def __init__(self, typ: TypeDesc, shape: tuple[int, ...]):
			assert isinstance(typ, _NumpyDesc)
			type = typ._type
			dims = 1
			self.shape = shape
			for i in shape:
				dims *= i
			TypeDesc.__init__(self, type.itemsize * dims, [type.itemsize * dims])
			self._type = type

		def get(self, slot: StorageSlot, off: int) -> np.ndarray:
			dat = slot.read(off, self.size)
			return np.frombuffer(dat, self._type).reshape(self.shape).copy()

		def set(self, slot: StorageSlot, off: int, val: np.ndarray):
			assert val.dtype == self._type
			mv = memoryview(val).cast('B')
			assert len(mv) == self.size, f'invalid len {len(mv)} vs expected {self.size}'
			slot.write(off, mv)

	class _NumpyDesc(TypeDesc):
		def __init__(self, typ: np.number):
			type = np.dtype(typ)
			TypeDesc.__init__(self, type.itemsize, [type.itemsize])
			self._type = type
			self._typ = typ

		def get(self, slot: StorageSlot, off: int):
			dat = slot.read(off, self.size)
			return np.frombuffer(dat, self._typ).reshape((1,))[0]

		def set(self, slot: StorageSlot, off: int, val):
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

	def make_ndarray(cls: '_Instantiation') -> TypeDesc | None:
		if cls.origin is not np.ndarray:
			return None

		assert len(cls.args) == 2
		shape = cls.args[0]
		assert isinstance(shape, LitTuple)
		assert all(
			isinstance(a, LitPy) and len(a.alts) == 1 and isinstance(a.alts[0], int)
			for a in shape.args
		)
		typ = cls.args[1]
		assert isinstance(typ, TypeDesc)
		return _NumpyNDDesc(
			typ, tuple(a.alts[0] for a in typing.cast(tuple[LitPy], shape.args))
		)

	global _imp
	_imp = make_ndarray


def try_handle_np(cls: '_Instantiation') -> TypeDesc | None:
	if _imp is None:
		return None
	return _imp(cls)


from .generate import _known_descs, _Instantiation, LitTuple, LitPy

if 'numpy' in sys.modules:
	_populate_np_descs()
else:
	import importlib.abc

	class _NpDetector(importlib.abc.MetaPathFinder):
		def find_spec(self, fullname, path, target=None):
			if fullname == 'numpy':
				sys.meta_path.remove(self)
				_populate_np_descs()

	sys.meta_path.insert(0, _NpDetector())
