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
	'allow',
	'inmem_allocate',
	'Root',
	'ROOT_SLOT_ID',
	'Slot',
	'Manager',
	'Indirection',
	'VLA',
)

from .vec import DynArray, Array
from .tree_map import TreeMap
from .root import Root

from ._internal.core import Indirection, VLA

from ._internal.core import ROOT_SLOT_ID, Slot, Manager, InmemManager

import typing

from ._internal.generate import allow

from ._internal.generate import (
	ORIGINAL_INIT_ATTR,
	generate_storage,
	_known_descs,
	_storage_build,
	_BuilderCtx,
)


def inmem_allocate[T](t: typing.Type[T], *init_args, **init_kwargs) -> T:
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


def copy_to_memory[T](val: T) -> T:
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
	_data: bytes

	def load(self) -> T:
		return pickle.loads(self._data)

	def store(self, val: T) -> None:
		self._data = pickle.dumps(val)
