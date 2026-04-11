"""
Persistent storage module for GenLayer contracts.

This module provides:
- ``DynArray``: Dynamic-length arrays
- ``Array``: Fixed-size arrays
- ``TreeMap``: Tree-based key-value storage
- ``allow_storage``: Decorator for storage-enabled classes
- ``inmem_allocate``: In-memory allocation utility
- ``Root``: Root storage class
"""

__all__ = (
	'DynArray',
	'Array',
	'TreeMap',
	'Comparable',
	'allow',
	'inmem_allocate',
	'Pickled',
	'Root',
	'ROOT_SLOT_ID',
	'Slot',
	'Manager',
	'Indirection',
	'VLA',
)

from .dyn_array import DynArray
from .array import Array
from .tree_map import TreeMap, Comparable
from .root import Root

from .core import Indirection, VLA

from .core import ROOT_SLOT_ID, Slot, Manager, InmemManager

import typing

from ._internal.generate import allow

from ._internal.generate import (
	ORIGINAL_INIT_ATTR,
	generate_storage,
	_known_descs,
	_storage_build,
	_BuilderCtx,
)


def inmem_allocate[T](t: typing.Type[T], /, *init_args, **init_kwargs) -> T:
	"""
	Allocate a storage type in memory (useful for testing).

	:param t: storage-allowed type to allocate
	:param init_args: positional arguments forwarded to ``__init__``
	:param init_kwargs: keyword arguments forwarded to ``__init__``
	:returns: new in-memory instance of the given type
	"""
	td = _storage_build(_BuilderCtx.empty(), t)
	man = InmemManager()

	instance = td.get(man.get_store_slot(ROOT_SLOT_ID), 0)

	init = getattr(td, 'cls', None)
	if init is None:
		init = getattr(t, '__init__', None)
	else:
		init = getattr(init, '__init__', None)
	if init is not None:
		if hasattr(init, ORIGINAL_INIT_ATTR):
			init = getattr(init, ORIGINAL_INIT_ATTR)
		init(instance, *init_args, **init_kwargs)

	return instance


def cast_slot[T](t: typing.Type[T], manager: Manager, slot: bytes, offset: int, /) -> T:
	"""
	Unsafely casts a storage slot to the given type. Use with caution.
	"""
	td = _storage_build(_BuilderCtx.empty(), t)

	instance = td.get(manager.get_store_slot(slot), offset)

	return instance


def copy_to_memory[T](val: T, /) -> T:
	"""
	Deep-copy a storage value into a new in-memory instance.

	:param val: storage-backed value to copy
	:returns: independent in-memory copy
	:raises AssertionError: when val is not a storage type
	"""
	# we know that val is a storage type
	td = getattr(val, '__type_desc__', None)
	assert td is not None

	man = InmemManager()
	slot = man.get_store_slot(ROOT_SLOT_ID)

	td.set(slot, 0, val)

	return td.get(slot, 0)


import pickle


@allow
class Pickled[T]:
	"""
	Storage wrapper that persists arbitrary Python objects via pickle serialization.
	"""

	_data: bytes

	def load(self) -> T:
		"""
		Deserialize and return the stored value.

		:returns: the unpickled object
		"""
		return pickle.loads(self._data)

	def store(self, val: T, /) -> None:
		"""
		Serialize and persist the given value.

		:param val: object to pickle and store
		"""
		self._data = pickle.dumps(val)
