import genlayer.wasi as wasi

from .types import *

import typing
import json
from types import SimpleNamespace
import base64

def public(f):
	setattr(f, '__public__', True)
	return f

class AlreadySerializedResult(str):
	def __new__(cls, *args, **kwargs):
		return str.__new__(cls, *args, **kwargs)

def account_from_b64(x: str) -> bytes:
	return base64.b64decode(x)

class ContractMethod:
	def __init__(self, addr: Address, name: str):
		self.addr = addr
		self.name = name
	def __call__(self, *args):
		obj = {
			"method": self.name,
			"args": args,
		}
		calldata = json.dumps(obj)
		res = wasi.call_contract(self.addr, calldata)
		return json.loads(res)

class Contract:
	def __init__(self, addr: Address):
		if not isinstance(addr, Address):
			raise Exception("address expected")
		self.addr = addr
	def __getattr__(self, name):
		return ContractMethod(self.addr, name)


message_raw = json.loads(wasi.get_message_data())

message = SimpleNamespace(
	gas=message_raw["gas"],
	contract_account=Address(message_raw["contract_account"]),
	sender_account=Address(message_raw["sender_account"]),
	value=message_raw.get("value", None),
)

def rollback(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)

class Runner:
	def run(self):
		pass

def run_nondet(eq_principle, runner: Runner) -> typing.Any:
	import pickle
	res = wasi.run_nondet(json.dumps(eq_principle), pickle.dumps(runner))
	res = base64.b64decode(res)
	return pickle.loads(res)
