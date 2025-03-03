__all__ = (
	'DynArray',
	'Array',
	'TreeMap',
	'VecDB',
	'allow_storage',
	'storage_inmem_allocate',
)

from .vec import DynArray, Array
from .tree_map import TreeMap
from .annotations import *

import typing


def storage_inmem_allocate[T](t: typing.Type[T], *init_args, **init_kwargs) -> T:
	from ._internal.generate import _storage_build, Lit
	from ._internal.core import _FakeStorageMan, ROOT_STORAGE_ADDRESS

	td = _storage_build(t, {})
	assert not isinstance(td, Lit)
	man = _FakeStorageMan()

	instance = td.get(man.get_store_slot(ROOT_STORAGE_ADDRESS), 0)

	init = getattr(td, 'cls', None)
	if init is None:
		init = getattr(t, '__init__', None)
	else:
		init = getattr(init, '__init__', None)
	if init is not None:
		if hasattr(init, '__original_init__'):
			init = init.__original_init__
		init(instance, *init_args, **init_kwargs)

	return instance


if typing.TYPE_CHECKING:
	from .vecdb import VecDB
else:
	import sys

	if 'numpy' in sys.modules:
		from .vecdb import VecDB
	else:

		def err():
			raise ImportError(
				'please import `numpy` before `from genlayer import *` if you wish to use VecDB'
			)

		class _VecDBMeta(type):
			def __getattr__(cls, name):
				err()

		class _VecDB(metaclass=_VecDBMeta):
			def __init__(self, *args, **kwargs):
				err()

			def __class_getitem__(cls, key):
				err()

		VecDB = _VecDB
