import io
import os
import typing

import genlayer.py.calldata as calldata
from genlayer.py.types import *

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
	entry_data: bytes
	entry_leader_data: bytes | None

if os.getenv('GENERATING_DOCS', 'false') == 'true':
	message_raw: MessageRawType = ... # type: ignore
else:
	message_raw = typing.cast(MessageRawType, calldata.decode(io.FileIO(0, closefd=False).readall()))
