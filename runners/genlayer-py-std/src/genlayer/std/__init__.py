import typing
import json
from types import SimpleNamespace as _SimpleNamespace

import genlayer.py.calldata as calldata

"""
reexport alias
"""
import genlayer.std.advanced as advanced

"""
reexport alias
"""

import genlayer.std._wasi as wasi

"""
reexport alias
"""

# reexports
from ..py.types import *
from .eq_principles import *
from .nondet_fns import *
from .genvm_contracts import *


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


message_raw = json.loads(wasi.get_message_data())
"""
Raw message as parsed json
"""

message = _SimpleNamespace(
	contract_account=Address(message_raw['contract_account']),
	sender_account=Address(message_raw['sender_account']),
	value=message_raw.get('value', None),
	is_init=message_raw.get('is_init', None),
)
"""
Represents fields from transaction message that was sent
"""


def rollback_immediate(reason: str) -> typing.NoReturn:
	"""
	Performs an immediate rollback, current VM won't be able to handle it, stack unwind will not happen
	"""
	wasi.rollback(reason)


def contract(t: type) -> type:
	"""
	Marks class as a contract

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
