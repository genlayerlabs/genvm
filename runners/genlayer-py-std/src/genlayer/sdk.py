import genlayer.wasi as wasi

import typing
import json
from types import SimpleNamespace as _SimpleNamespace
import base64

# reexports
import genlayer.py.calldata as calldata
from .py.types import *
from .asyn import *

def public(f):
	setattr(f, '__public__', True)
	return f

class AlreadySerializedResult(bytes):
	def __new__(cls, *args, **kwargs):
		return bytes.__new__(cls, *args, **kwargs)

def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)

def _decode_sub_vm_result(data: bytes) -> typing.Any:
	mem = memoryview(data)
	if mem[0] != 0:
		raise Rollback(str(mem[1:], encoding='utf8'))
	return calldata.decode(mem[1:])

class OtherContractMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name
	def __call__(self, *args) -> AwaitableResultMap[typing.Any]:
		obj = {
			"method": self.name,
			"args": args,
		}
		cd = calldata.encode(obj)
		res = wasi.call_contract(self.addr.as_bytes, cd)
		return AwaitableResultMap(res, _decode_sub_vm_result)

class OtherContract:
	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception("address expected")
		self.addr = addr
	def __getattr__(self, name):
		return OtherContractMethod(self.addr, name)

message_raw = json.loads(wasi.get_message_data())

message = _SimpleNamespace(
	gas=message_raw["gas"],
	contract_account=Address(message_raw["contract_account"]),
	sender_account=Address(message_raw["sender_account"]),
	value=message_raw.get("value", None),
	is_init=message_raw.get("is_init", None),
)

def rollback_immediate(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)

class Runner:
	def run(self):
		pass

def get_webpage(config: typing.Any, url: str) -> AwaitableResultStr:
	return AwaitableResultStr(wasi.get_webpage(json.dumps(config), url))

def call_llm(config: typing.Any, prompt: str) -> AwaitableResultStr:
	return AwaitableResultStr(wasi.call_llm(json.dumps(config), prompt))

def run_nondet(eq_principle, runner: Runner) -> AwaitableResultMap[typing.Any]:
	import pickle
	res = wasi.run_nondet(json.dumps(eq_principle), pickle.dumps(runner))
	return AwaitableResultMap[typing.Any](res, _decode_sub_vm_result)

def contract(t: type) -> type:
	import genlayer.runner as runner
	import inspect
	mod = inspect.getmodule(t)
	if mod is None:
		raise Exception(f"can't detect module where {t} is declared")
	if hasattr(mod, '__KNOWN_CONTRACT'):
		raise Exception(f"only one @contract is allowed, old {mod.__KNOWN_CONTRACT} new {t}")
	t.__contract__ = True
	from genlayer.py.storage import storage
	t = storage(t)
	setattr(mod, '__KNOWN_CONTRACT', t)
	return t
