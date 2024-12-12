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
	'contract',
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
	'sandbox',
)

import typing
import json
import os

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


class public:
	@staticmethod
	def view(f):
		"""
		Decorator that marks a contract method as a public view
		"""
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', True)
		return f

	@staticmethod
	def write(f):
		"""
		Decorator that marks a contract method as a public write
		"""
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', False)
		return f


class MessageType(typing.NamedTuple):
	contract_account: Address
	"""
	Address of current Intelligent Contract
	"""
	sender_account: Address
	"""
	Address of this call initiator
	"""
	origin_account: Address
	"""
	Entire transaction initiator
	"""
	value: int | None
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
		contract_account=Address(message_raw['contract_account']),
		sender_account=Address(message_raw['sender_account']),
		origin_account=Address(message_raw['origin_account']),
		value=message_raw.get('value', None),
		is_init=message_raw.get('is_init', None),
		chain_id=u256(int(message_raw['chain_id'])),
	)


def rollback_immediate(reason: str) -> typing.NoReturn:
	"""
	Performs an immediate rollback, current VM won't be able to handle it, stack unwind will not happen
	"""
	wasi.rollback(reason)


def contract(t: type) -> type:
	"""
	Decorator that marks class as a contract

	.. note::
		There can be only one "contract" at address, so this function must be called at least once
	"""
	import inspect

	mod = inspect.getmodule(t)
	if mod is None:
		raise Exception(f"can't detect module where {t} is declared")
	if hasattr(mod, '__KNOWN_CONTRACT'):
		raise Exception(
			f'only one @contract is allowed, old {mod.__KNOWN_CONTRACT} new {t}'
		)
	t.__contract__ = True
	from genlayer.py.storage._internal.generate import storage

	t = storage(t)
	setattr(mod, '__KNOWN_CONTRACT', t)
	return t
