"""
Blockchain specific functionality, that won't work without GenVM
"""

__all__ = (
	'Lazy',
	'MessageType',
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
)

import typing
import json
import os

import genlayer.py.eth as eth
import genlayer.py.calldata as calldata
import genlayer.std.advanced as advanced
import genlayer.std._wasi as wasi

# reexports
from ..py.types import *
from .eq_principles import *
from .nondet_fns import *
from .genvm_contracts import *
from .eth import *


def private(f):
	"""
	Decorator that marks method as private. As all methods are private by default it does nothing.
	"""
	return f


class _write:
	def payable[T](self, f: T) -> T:
		self(f)
		setattr(f, '__payable__', True)
		return f

	def __call__[T](self, f: T) -> T:
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', False)
		return f


class public:
	@staticmethod
	def view(f):
		"""
		Decorator that marks a contract method as a public view
		"""
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', True)
		return f

	write = _write()
	"""
	Decorator that marks a contract method as a public write. Has `.payable`

	.. code:: python

		@gl.public.write
		def foo(self) -> None: ...

		@gl.public.write.payable
		def bar(self) -> None: ...
	"""


del _write


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
	is_init: bool
	"""
	``True`` *iff* it is a deployment
	"""
	chain_id: u256
	"""
	Current chain ID
	"""


if os.getenv('GENERATING_DOCS', 'false') == 'true':
	message_raw: dict = ...  # type: ignore
	"""
	Raw message as parsed json
	"""

	message: MessageType = ...  # type: ignore
	"""
	Represents fields from a transaction message that was sent
	"""
else:
	message_raw = json.loads(wasi.get_message_data())

	message = MessageType(
		contract_address=Address(message_raw['contract_address']),
		sender_address=Address(message_raw['sender_address']),
		origin_address=Address(message_raw['origin_address']),
		value=u256(message_raw.get('value', None) or 0),
		is_init=message_raw.get('is_init', None),
		chain_id=u256(int(message_raw['chain_id'])),
	)


def rollback_immediate(reason: str) -> typing.NoReturn:
	"""
	Performs an immediate rollback, current VM won't be able to handle it, stack unwind will not happen
	"""
	wasi.rollback(reason)
