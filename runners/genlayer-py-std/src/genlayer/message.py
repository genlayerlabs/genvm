__all__ = (
	'MessageRawType',
	'raw',
)

import io
import os
import typing

import genlayer.calldata as calldata
from genlayer.types import u256, Address


class MessageRawType(typing.TypedDict):
	contract_address: Address
	"""
	Address of current Intelligent Contract
	"""

	sender_address: Address
	"""
	Address of this call initiator
	"""

	origin_address: Address
	"""
	Entire transaction initiator
	"""

	stack: list[Address]
	"""
	Stack of view method calls, excluding last (``contract_address``)
	"""

	value: u256

	datetime: str
	"""
	Transaction datetime. For ``#get-schema`` it can be some predefined datetime
	"""

	is_init: bool
	"""
	``True`` *iff* it is a deployment
	"""

	chain_id: u256
	"""
	Current chain ID
	"""

	entry_kind: int
	"""
	One of:
		- ``0`` for ``MAIN``
		- ``1`` for ``SANDBOX``
		- ``2`` for ``CONSENSUS_STAGE``

	See :ref:`startup-process-reference` for more details.
	"""
	entry_data: bytes
	entry_stage_data: calldata.Encodable


contract_address: Address = ...  # type: ignore
"""Address of current Intelligent Contract"""

sender_address: Address = ...  # type: ignore
"""Address of this call initiator"""

origin_address: Address = ...  # type: ignore
"""Entire transaction initiator"""

stack: list[Address] = ...  # type: ignore

value: u256 = ...  # type: ignore
"""Value sent with the transaction"""

chain_id: u256 = ...  # type: ignore
"""Current chain ID"""

is_init: bool = ...  # type: ignore

entry_kind: int = ...  # type: ignore

entry_data: bytes = ...  # type: ignore

entry_stage_data: calldata.Encodable = ...  # type: ignore

if os.getenv('GENERATING_DOCS', 'false') == 'true':
	raw: MessageRawType = ...  # type: ignore
else:
	raw = typing.cast(
		MessageRawType, calldata.decode(io.FileIO(0, closefd=False).readall())
	)

	globals().update(raw)
