__all__ = (
	'DynArray',
	'Array',
	'TreeMap',
	'VecDB',
)

from .vec import DynArray, Array
from .tree_map import TreeMap

import typing

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
