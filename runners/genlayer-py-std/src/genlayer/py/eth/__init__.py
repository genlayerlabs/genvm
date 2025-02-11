"""
This module is responsible for interactions with ghost/external contracts
"""

__all__ = (
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
