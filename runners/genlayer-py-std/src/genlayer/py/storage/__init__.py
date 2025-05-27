__all__ = (
	'DynArray',
	'Array',
	'TreeMap',
	'allow_storage',
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
from .annotations import *
from .root import Root

from ._internal.core import Indirection, VLA

from ._internal.core import ROOT_SLOT_ID, Slot, Manager, InmemManager

import typing

from ._internal.generate import (
	ORIGINAL_INIT_ATTR,
	generate_storage,
	_known_descs,
	_storage_build,
	Lit,
)


def inmem_allocate[T](t: typing.Type[T], *init_args, **init_kwargs) -> T:
	td = _storage_build(t, {})
	assert not isinstance(td, Lit)
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
