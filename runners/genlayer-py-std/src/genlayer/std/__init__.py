import genlayer._wasi as wasi

import typing
import json
from types import SimpleNamespace as _SimpleNamespace
import base64
import importlib

advanced = importlib.import_module('.advanced', __name__)

# reexports
import genlayer.py.calldata as calldata
from ..py.types import *
from ._private import _decode_sub_vm_result, _lazy_from_fd


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


def _make_calldata_obj(method, args, kwargs):
	ret = {'method': method}
	if len(args) > 0:
		ret.update({'args': args})
	if len(kwargs) > 0:
		ret.update({'kwargs': kwargs})
	return ret


class _ContractAtViewMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name

	def __call__(self, *args, **kwargs) -> typing.Any:
		return self.lazy(*args, **kwargs).get()

	def lazy(self, *args, **kwargs) -> Lazy[typing.Any]:
		obj = _make_calldata_obj(self.name, args, kwargs)
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
		obj = _make_calldata_obj(self.name, args, kwargs)
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
	contract_account=Address(message_raw['contract_account']),
	sender_account=Address(message_raw['sender_account']),
	value=message_raw.get('value', None),
	is_init=message_raw.get('is_init', None),
)


def rollback_immediate(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)


class _LazyApi[T, **R]:
	def __init__(self, fn: typing.Callable[R, Lazy[T]]):
		self.fn = fn

	def __call__(self, *args: R.args, **kwargs: R.kwargs) -> T:
		return self.fn(*args, **kwargs).get()

	def lazy(self, *args: R.args, **kwargs: R.kwargs) -> Lazy[T]:
		return self.fn(*args, **kwargs)


class _GetWebpageConfig(typing.TypedDict):
	mode: typing.Literal['html', 'text']


def _get_webpage(url: str, **config: typing.Unpack[_GetWebpageConfig]) -> Lazy[str]:
	return _lazy_from_fd(
		wasi.get_webpage(json.dumps(config), url), lambda buf: str(buf, 'utf-8')
	)


get_webpage = _LazyApi(_get_webpage)
del _get_webpage


class _ExecPromptConfig(typing.TypedDict):
	pass


def _exec_prompt(prompt: str, **config: typing.Unpack[_ExecPromptConfig]) -> Lazy[str]:
	return _lazy_from_fd(
		wasi.exec_prompt(json.dumps(config), prompt), lambda buf: str(buf, 'utf-8')
	)


exec_prompt = _LazyApi(_exec_prompt)
del _exec_prompt


def _eq_principle_strict_eq[T](fn: typing.Callable[[], T]) -> Lazy[T]:
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

	return advanced.run_nondet(fn, validator_fn)


eq_principle_strict_eq = _LazyApi(_eq_principle_strict_eq)
del _eq_principle_strict_eq


def _eq_principle_prompt_comparative(
	fn: typing.Callable[[], str], principle: str
) -> Lazy[str]:
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

	return advanced.run_nondet(fn, validator_fn)


eq_principle_prompt_comparative = _LazyApi(_eq_principle_prompt_comparative)
del _eq_principle_prompt_comparative


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
