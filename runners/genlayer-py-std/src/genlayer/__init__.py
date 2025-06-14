"""
Common import for all contracts

It exposes most of the types to the top scope and encapsulates other utility under :py:obj:`gl` namespace which is a proxy to :py:mod:`genlayer.gl`
"""

__all__ = (
	'gl',
	'Address',
	'allow_storage',
	'Array',
	'DynArray',
	'Keccak256',
	'TreeMap',
	'bigint',
	'i104',
	'i112',
	'i120',
	'i128',
	'i136',
	'i144',
	'i152',
	'i16',
	'i160',
	'i168',
	'i176',
	'i184',
	'i192',
	'i200',
	'i208',
	'i216',
	'i224',
	'i232',
	'i24',
	'i240',
	'i248',
	'i256',
	'i32',
	'i40',
	'i48',
	'i56',
	'i64',
	'i72',
	'i8',
	'i80',
	'i88',
	'i96',
	'u104',
	'u112',
	'u120',
	'u128',
	'u136',
	'u144',
	'u152',
	'u16',
	'u160',
	'u168',
	'u176',
	'u184',
	'u192',
	'u200',
	'u208',
	'u216',
	'u224',
	'u232',
	'u24',
	'u240',
	'u248',
	'u256',
	'u32',
	'u40',
	'u48',
	'u56',
	'u64',
	'u72',
	'u8',
	'u80',
	'u88',
	'u96',
)

import os

from .py.types import *
from .py.storage import *

_gen_docs = os.getenv('GENERATING_DOCS', 'false') == 'true'

if not typing.TYPE_CHECKING and not _gen_docs:

	class GL:
		"""
		proxy to :py:mod:`genlayer.gl` used for lazy loading
		"""

		def __getattr__(self, attr):
			globals().pop('gl', None)
			import genlayer.gl as _imp

			# below is needed to trick cloudpickle
			globals()['gl'] = _imp

			return getattr(_imp, attr)

	gl = GL()
	del GL
else:
	import genlayer.gl as gl
