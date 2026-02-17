"""
Transaction message context module.

This module provides access to the current transaction message context:
- ``contract_address``: Address of current Intelligent Contract
- ``sender_address``: Address of this call initiator
- ``origin_address``: Entire transaction initiator
- ``value``: Value sent with the transaction
- ``chain_id``: Current chain ID
- ``raw``: Full raw message with all fields

For complete raw message access, use ``genlayer.message.raw`` which includes
additional fields like ``stack``, ``datetime``, ``is_init``, ``entry_kind``, etc.
"""

__all__ = (
	'RawType',
	'raw',
	'contract_address',
	'sender_address',
	'origin_address',
	'value',
	'chain_id',
)

import os
import typing

from genlayer._internal.msg import MessageRawType as RawType, message_raw as raw
from genlayer.types import Address, u256

if os.getenv('GENERATING_DOCS', 'false') == 'true':
	contract_address: Address = ...  # type: ignore
	"""Address of current Intelligent Contract"""

	sender_address: Address = ...  # type: ignore
	"""Address of this call initiator"""

	origin_address: Address = ...  # type: ignore
	"""Entire transaction initiator"""

	value: u256 = ...  # type: ignore
	"""Value sent with the transaction"""

	chain_id: u256 = ...  # type: ignore
	"""Current chain ID"""
else:
	contract_address: Address = raw['contract_address']
	sender_address: Address = raw['sender_address']
	origin_address: Address = raw['origin_address']
	value: u256 = raw['value']
	chain_id: u256 = raw['chain_id']
