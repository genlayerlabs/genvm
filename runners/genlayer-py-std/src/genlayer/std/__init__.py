"""
Blockchain specific functionality, that won't work without GenVM
and reexports form :py:mod:`genlayer.py` provided for convenience
"""

__all__ = (
	'Lazy',
	'MessageType',
	'MessageRawType',
	'wasi',
	'advanced',
	'calldata',
	'private',
	'public',
	'Contract',
	'contract_interface',
	'ContractAt',
	'deploy_contract',
	'eth_contract',
	'eq_principle_prompt_comparative',
	'eq_principle_prompt_non_comparative',
	'eq_principle_strict_eq',
	'eq_principles',
	'exec_prompt',
	'get_webpage',
	'message',
	'message_raw',
	'rollback_immediate',
	'eth',
	'storage_inmem_allocate',
	'Event',
)

import typing
import json
import os

import genlayer.py.eth as eth
import genlayer.py.calldata as calldata
import genlayer.std.advanced as advanced
import genlayer.std._wasi as wasi

from .events import Event

# reexports
from ..py.types import *
from ..py.storage import storage_inmem_allocate
from .eq_principles import *
from .nondet_fns import *
from .genvm_contracts import *
from .eth import *
from .annotations import *
from .msg import MessageRawType, message_raw as _message_raw_original

import genlayer.std._internal.gl_call as gl_call


class MessageType(typing.NamedTuple):
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
	value: u256
	chain_id: u256
	"""
	Current chain ID
	"""


if os.getenv('GENERATING_DOCS', 'false') == 'true':
	message_raw: MessageRawType = ...  # type: ignore
	"""
	Raw message as parsed json
	"""

	message: MessageType = ...  # type: ignore
	"""
	Represents fields from a transaction message that was sent
	"""
else:
	message_raw = _message_raw_original
	message = MessageType(
		contract_address=message_raw['contract_address'],
		sender_address=message_raw['sender_address'],
		origin_address=message_raw['origin_address'],
		value=u256(message_raw['value']),
		chain_id=u256(message_raw['chain_id']),
	)


def rollback_immediate(reason: str) -> typing.NoReturn:
	"""
	Performs an immediate rollback, current VM won't be able to handle it, stack unwind will not happen
	"""

	gl_call.rollback(reason)
