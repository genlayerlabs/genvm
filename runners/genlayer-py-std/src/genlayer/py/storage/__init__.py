__all__ = (
	'DynArray',
	'Array',
	'TreeMap',
	'allow_storage',
	'storage_inmem_allocate',
)

from .vec import DynArray, Array
from .tree_map import TreeMap
from .annotations import *

import typing

from ._internal.generate import ORIGINAL_INIT_ATTR


def storage_inmem_allocate[T](t: typing.Type[T], *init_args, **init_kwargs) -> T:
	from ._internal.generate import _storage_build, Lit
	from ._internal.core import _FakeStorageMan, ROOT_SLOT_ID

	td = _storage_build(t, {})
	assert not isinstance(td, Lit)
	man = _FakeStorageMan()

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
