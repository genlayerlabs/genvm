import genlayer.wasi as wasi

import typing
import json
from types import SimpleNamespace as _SimpleNamespace
import base64
import genlayer.advanced

# reexports
import genlayer.py.calldata as calldata
from .py.types import *
from ._private import _decode_sub_vm_result, _lazy_from_fd


def private(f):
	return f


def public(f):
	setattr(f, '__public__', True)
	return f


def _public_view(f):
	f = public(f)
	setattr(f, '__readonly__', True)
	return f


public.view = _public_view


def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)


class _ContractAtViewMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name

	def __call__(self, *args, **kwargs) -> Lazy[typing.Any]:
		obj = {
			'method': self.name,
			'args': args,
			'kwargs': kwargs,
		}
		cd = calldata.encode(obj)
		return _lazy_from_fd(
			wasi.call_contract(self.addr.as_bytes, cd), _decode_sub_vm_result
		)


class _ContractAtEmitMethod:
	def __init__(self, addr: Address, name: str, gas: int, code: bytes):
		self.addr = addr
		self.name = name
		self.gas = gas
		self.code = code

	def __call__(self, *args, **kwargs) -> None:
		obj = {
			'method': self.name,
			'args': args,
			'kwargs': kwargs,
		}
		cd = calldata.encode(obj)
		wasi.post_message(self.addr.as_bytes, cd, self.gas, self.code)


class ContractAt:
	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception('address expected')
		self.addr = addr

	def view(self):
		return _ContractAtView(self.addr)

	def emit(self, *, gas: int, code: bytes = b''):
		return _ContractAtEmit(self.addr, gas, code)


class _ContractAtView:
	def __init__(self, addr: Address):
		self.addr = addr

	def __getattr__(self, name):
		return _ContractAtViewMethod(self.addr, name)


class _ContractAtEmit:
	def __init__(self, addr: Address, gas: int, code: bytes):
		self.addr = addr
		self.gas = gas
		self.code = code

	def __getattr__(self, name):
		return _ContractAtEmitMethod(self.addr, name, self.gas, self.code)


message_raw = json.loads(wasi.get_message_data())

message = _SimpleNamespace(
	gas=message_raw['gas'],
	contract_account=Address(message_raw['contract_account']),
	sender_account=Address(message_raw['sender_account']),
	value=message_raw.get('value', None),
	is_init=message_raw.get('is_init', None),
)


def rollback_immediate(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)


def get_webpage(config: typing.Any, url: str) -> Lazy[str]:
	return _lazy_from_fd(
		wasi.get_webpage(json.dumps(config), url), lambda buf: str(buf, 'utf-8')
	)


def exec_prompt(config: typing.Any, prompt: str) -> Lazy[str]:
	return _lazy_from_fd(
		wasi.exec_prompt(json.dumps(config), prompt), lambda buf: str(buf, 'utf-8')
	)


def eq_principle_refl[T](fn: typing.Callable[[], T]) -> Lazy[T]:
	def validator_fn(leaders: typing.Any | Rollback) -> bool:
		try:
			my_res = fn()
		except Rollback as r:
			if isinstance(leaders, Rollback) and leaders.msg == r.msg:
				return True
		else:
			if not isinstance(leaders, Rollback) and leaders == my_res:
				return True
		return False

	return genlayer.advanced.run_nondet(fn, validator_fn)


def eq_principle_prompt(principle: str, fn: typing.Callable[[], str]) -> Lazy[str]:
	def validator_fn(leaders: typing.Any | Rollback) -> bool:
		if not isinstance(leaders, (str, Rollback)):
			raise Exception(f'invalid leaders result {leaders}')
		try:
			my_res = fn()
		except Rollback as r:
			if not isinstance(leaders, Rollback):
				return False
			return leaders.msg == r.msg
		if isinstance(leaders, Rollback):
			return False
		config = {
			'leader_answer': leaders,
			'validator_answer': my_res,
			'principle': principle,
		}
		return wasi.eq_principle_prompt(json.dumps(config))

	return genlayer.advanced.run_nondet(fn, validator_fn)


def contract(t: type) -> type:
	import genlayer.runner as runner
	import inspect

	mod = inspect.getmodule(t)
	if mod is None:
		raise Exception(f"can't detect module where {t} is declared")
	if hasattr(mod, '__KNOWN_CONTRACT'):
		raise Exception(
			f'only one @contract is allowed, old {mod.__KNOWN_CONTRACT} new {t}'
		)
	t.__contract__ = True
	from genlayer.py.storage import storage

	t = storage(t)
	setattr(mod, '__KNOWN_CONTRACT', t)
	return t
