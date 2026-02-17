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
- Advanced operations via ``gl.advanced`` (alias to ``gl.vm``)
- Method decorators via ``gl.public`` and ``gl.private``
"""

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
	'advanced',
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
	'allow',
)

# Add all type names to __all__
import genlayer.types as _types_mod

__all__ = __all__ + _types_mod.__all__

_gen_docs = os.getenv('GENERATING_DOCS', 'false') == 'true'

if typing.TYPE_CHECKING or _gen_docs:
	# For type checking and docs, import modules eagerly
	from . import contract
	from . import message
	from . import vm
	from . import evm
	from . import nondet
	from . import eq_principle
	from . import types
	from . import calldata
	from . import storage
	import _genlayer_wasi as wasi

	advanced = vm
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
		'advanced': 'genlayer.vm',
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
