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

from .calldata import *
from .support import *
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
