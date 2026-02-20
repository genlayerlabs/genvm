"""
GenLayer Python Standard Library

The recommended import pattern is:

.. code:: python

	import genlayer as gl

This provides access to:
- Type aliases: ``gl.u8``, ``gl.u16``, ..., ``gl.u256``, ``gl.Address``, etc.
- Storage types: ``gl.TreeMap``, ``gl.DynArray``, ``gl.Array``
- Contract declaration via ``gl.contract.Contract``
- Contract interaction via ``gl.contract.interface``, ``gl.contract.deploy``, ``gl.contract.get_at``
- Message context via ``gl.message.contract_address``, ``gl.message.sender_address``, etc.
- VM operations via ``gl.vm``
- Non-deterministic operations via ``gl.nondet``
- Equivalence principles via ``gl.eq_principle``
- EVM interaction via ``gl.evm``
- Method decorators via ``gl.public`` and ``gl.private``
"""

IS_IN_VM = False

try:
	import _genlayer_wasi

	IS_IN_VM = True
except ImportError:
	pass

import os
import typing

# Pre-load storage to resolve circular dependency: reflect <-> storage
import genlayer.storage  # noqa: F401

# Re-export types and storage names so they are accessible as gl.X
from .types import *
from .storage import DynArray, Array, TreeMap, allow

# Decorators - directly import so gl.public and gl.private work
from ._internal.annotations import public, private

__all__ = (
	# Submodules (accessible via gl.X when using `import genlayer as gl`)
	'contract',
	'message',
	'vm',
	'evm',
	'nondet',
	'eq_principle',
	'types',
	'calldata',
	'storage',
	'wasi',
	# Decorators (accessible via gl.public, gl.private)
	'public',
	'private',
	# Storage types
	'DynArray',
	'Array',
	'TreeMap',
	# Unsigned integers
	'u8',
	'u16',
	'u24',
	'u32',
	'u40',
	'u48',
	'u56',
	'u64',
	'u72',
	'u80',
	'u88',
	'u96',
	'u104',
	'u112',
	'u120',
	'u128',
	'u136',
	'u144',
	'u152',
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
	'u240',
	'u248',
	'u256',
	# Signed integers
	'i8',
	'i16',
	'i24',
	'i32',
	'i40',
	'i48',
	'i56',
	'i64',
	'i72',
	'i80',
	'i88',
	'i96',
	'i104',
	'i112',
	'i120',
	'i128',
	'i136',
	'i144',
	'i152',
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
	'i240',
	'i248',
	'i256',
	# Other types
	'bigint',
	'Lazy',
	'Address',
	'SizedArray',
	'Keccak256',
)

_gen_docs = os.getenv('GENERATING_DOCS', 'false') == 'true'

if typing.TYPE_CHECKING or _gen_docs:
	# For type checking and docs, import modules eagerly
	from . import (
		contract,
		message,
		vm,
		evm,
		nondet,
		eq_principle,
		types,
		calldata,
		storage,
	)
	import _genlayer_wasi as wasi
else:
	# For runtime, use lazy loading to avoid circular imports and improve startup
	_lazy_modules = {
		'contract': 'genlayer.contract',
		'message': 'genlayer.message',
		'vm': 'genlayer.vm',
		'evm': 'genlayer.evm',
		'nondet': 'genlayer.nondet',
		'eq_principle': 'genlayer.eq_principle',
		'types': 'genlayer.types',
		'calldata': 'genlayer.calldata',
		'storage': 'genlayer.storage',
	}

	def __getattr__(name: str):
		if name == 'wasi':
			import _genlayer_wasi

			globals()['wasi'] = _genlayer_wasi
			return _genlayer_wasi

		module_path = _lazy_modules.get(name)
		if module_path is not None:
			mod = __import__(module_path, fromlist=[name])
			globals()[name] = mod
			return mod

		raise AttributeError(f"module 'genlayer' has no attribute '{name}'")
