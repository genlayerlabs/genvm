import genlayer.std._wasi as wasi

import typing
import json
from types import SimpleNamespace as _SimpleNamespace
import base64

# reexport short aliases
import genlayer.py.calldata as calldata
import genlayer.std.advanced as advanced

# reexports
from ..py.types import *
from .eq_principles import *
from .nondet_fns import *
from .genvm_contracts import *


def private(f):
	return f


class public:
	@staticmethod
	def view(f):
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', True)
		return f

	@staticmethod
	def write(f):
		setattr(f, '__public__', True)
		setattr(f, '__readonly__', False)
		return f


def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)


message_raw = json.loads(wasi.get_message_data())

message = _SimpleNamespace(
	contract_account=Address(message_raw['contract_account']),
	sender_account=Address(message_raw['sender_account']),
	value=message_raw.get('value', None),
	is_init=message_raw.get('is_init', None),
)


def rollback_immediate(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)


def contract(t: type) -> type:
	import inspect

	mod = inspect.getmodule(t)
	if mod is None:
		raise Exception(f"can't detect module where {t} is declared")
	if hasattr(mod, '__KNOWN_CONTRACT'):
		raise Exception(
			f'only one @contract is allowed, old {mod.__KNOWN_CONTRACT} new {t}'
		)
	t.__contract__ = True
	from genlayer.py.storage.generate import storage

	t = storage(t)
	setattr(mod, '__KNOWN_CONTRACT', t)
	return t
