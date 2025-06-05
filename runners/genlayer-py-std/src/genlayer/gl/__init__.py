"""
Blockchain specific functionality, that won't work without GenVM
and reexports form :py:mod:`genlayer.py` provided for convenience
"""

__all__ = (
	'Lazy',
	'Event',
	'MessageType',
	'MessageRawType',
	'message',
	'message_raw',
	'private',
	'public',
	# auto-loaded modules
	'wasi',
	'calldata',
	'storage',
	# lazy loaded modules
	'evm',
	'nondet',
	'eq_principle',
	'contract_interface',
	'deploy_contract',
	'Contract',
	'get_contract_at',
)

import typing
import os

import genlayer.py.calldata as calldata
import _genlayer_wasi as wasi

from .events import Event

# reexports
from ..py.types import *
import genlayer.py.storage as storage

from ._internal.storage import STORAGE_MAN

storage.Root.MANAGER = STORAGE_MAN


def _post_load_evm(mod):
	from ._internal.eth import evm_contract_interface as ci

	mod.contract_interface = ci


if typing.TYPE_CHECKING or os.getenv('GENERATING_DOCS', 'false') == 'true':
	import genlayer.gl.nondet as nondet
	import genlayer.gl.advanced as advanced
	import genlayer.gl.eq_principle as eq_principle
	import genlayer.py.evm as evm
else:
	_modules = {
		'nondet': ('genlayer.gl.nondet', None),
		'eq_principle': ('genlayer.gl.eq_principle', None),
		'evm': ('genlayer.py.evm', _post_load_evm),
		'advanced': ('genlayer.gl.advanced', None),
	}

	def __getattr__(name: str):
		module, post_load = _modules.get(name, (None, None))
		if module is None:
			raise AttributeError(f"module 'genlayer.gl' has no attribute '{name}'")
		mod = __import__(module, fromlist=[name])
		if post_load is not None:
			post_load(mod)
		globals()[name] = mod
		return mod


from .genvm_contracts import *
from .annotations import *
from .msg import MessageRawType, message_raw as _message_raw_original


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
