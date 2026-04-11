"""
EVM (Ethereum Virtual Machine) contract interaction module.

This module provides functionality for interacting with EVM-compatible contracts:
- ``contract_interface``: Decorator for creating type-safe EVM contract interfaces
- ABI encoding/decoding utilities
- Fixed-size byte types (bytes1 through bytes32)
"""

__all__ = (
	'contract_interface',
	'signature_of',
	'type_name_of',
	'selector_of',
	'MethodEncoder',
	'encode',
	'decode',
	'ContractProxy',
	'ContractDeclaration',
	'InplaceTuple',
	'bytes1',
	'bytes2',
	'bytes3',
	'bytes4',
	'bytes5',
	'bytes6',
	'bytes7',
	'bytes8',
	'bytes9',
	'bytes10',
	'bytes11',
	'bytes12',
	'bytes13',
	'bytes14',
	'bytes15',
	'bytes16',
	'bytes17',
	'bytes18',
	'bytes19',
	'bytes20',
	'bytes21',
	'bytes22',
	'bytes23',
	'bytes24',
	'bytes25',
	'bytes26',
	'bytes27',
	'bytes28',
	'bytes29',
	'bytes30',
	'bytes31',
	'bytes32',
)

import typing


class InplaceTuple:
	# editorconfig-checker-disable
	"""
	This class indicates that tuple should be encoded/decoded in-place.
	Which means that even if it is dynamically sized, it is ignored.
	It is useful for encoding/decoding arguments and returns

	.. code-block:: python

	        tuple[InplaceTuple, str, u256]
	"""

	# editorconfig-checker-enable

	__slots__ = ()


bytes1 = typing.NewType('bytes1', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes2 = typing.NewType('bytes2', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes3 = typing.NewType('bytes3', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes4 = typing.NewType('bytes4', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes5 = typing.NewType('bytes5', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes6 = typing.NewType('bytes6', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes7 = typing.NewType('bytes7', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes8 = typing.NewType('bytes8', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes9 = typing.NewType('bytes9', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes10 = typing.NewType('bytes10', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes11 = typing.NewType('bytes11', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes12 = typing.NewType('bytes12', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes13 = typing.NewType('bytes13', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes14 = typing.NewType('bytes14', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes15 = typing.NewType('bytes15', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes16 = typing.NewType('bytes16', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes17 = typing.NewType('bytes17', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes18 = typing.NewType('bytes18', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes19 = typing.NewType('bytes19', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes20 = typing.NewType('bytes20', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes21 = typing.NewType('bytes21', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes22 = typing.NewType('bytes22', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes23 = typing.NewType('bytes23', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes24 = typing.NewType('bytes24', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes25 = typing.NewType('bytes25', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes26 = typing.NewType('bytes26', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes27 = typing.NewType('bytes27', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes28 = typing.NewType('bytes28', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes29 = typing.NewType('bytes29', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes30 = typing.NewType('bytes30', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes31 = typing.NewType('bytes31', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""
bytes32 = typing.NewType('bytes32', bytes)
"""
Fixed-size byte array. These types are used for encoding/decoding fixed-size byte arrays in EVM contracts
"""


from .calldata import *
from .generate import contract_generator, ContractProxy, ContractDeclaration

import typing
from ..types import Address, u256

if typing.TYPE_CHECKING:
	from genlayer._internal.on_chain.eth import (
		evm_contract_interface as contract_interface,
	)


def __getattr__(name):
	if name == 'contract_interface':
		from genlayer._internal.on_chain.eth import (
			evm_contract_interface as contract_interface,
		)

		globals()['contract_interface'] = contract_interface
		return contract_interface
	raise AttributeError(f'module {__name__!r} has no attribute {name!r}')


import genlayer.chain


class IAccount(genlayer.chain.IAccount, typing.Protocol):
	def emit_call(self, value: u256, data: bytes) -> None: ...


class Account(IAccount, genlayer.chain.Account):
	def emit_value(self, value: u256, data: bytes, /) -> None:
		from genlayer._internal.on_chain.eth import perform_send

		perform_send(self.address, data, value)
