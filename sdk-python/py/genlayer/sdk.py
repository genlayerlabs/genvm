import genlayer.wasi as wasi
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

message_raw = json.loads(wasi.get_message_data())

message = SimpleNamespace(
	gas=message_raw["gas"],
	contract_account=base64.b64decode(message_raw["contract_account"]),
	sender_account=base64.b64decode(message_raw["sender_account"]),
	value=message_raw.get("value", None),
)

def rollback(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)

def _give_result(res):
	if res is None:
		exit(0)
	elif isinstance(res, AlreadySerializedResult):
		wasi.contract_return(res)
	else:
		wasi.contract_return(json.dumps(res))

def run(mod):
	entrypoint: bytes = wasi.get_entrypoint()
	CALL = b'call!'
	NONDET = b'nondet!'
	if entrypoint.startswith(CALL):
		calldata = json.loads(entrypoint[len(CALL):].decode())
		meth = getattr(mod, calldata['method'])
		if not getattr(meth, '__public__', False):
			raise Exception(f"can't call non-public methods")
		res = meth(*calldata['args'])
		_give_result(res)
	elif entrypoint.startswith(NONDET):
		import pickle
		res = pickle.loads(entrypoint[len(NONDET):])
		res = res.run()
		wasi.contract_return(base64.b64encode(pickle.dumps(res)).decode('ascii'))
	else:
		raise Exception(f"unknown entrypoint {entrypoint}")

class Runner:
	def run(self):
		pass

def run_nondet(eq_principle, runner: Runner) -> typing.Any:
	import pickle
	res = wasi.run_nondet(json.dumps(eq_principle), pickle.dumps(runner))
	res = base64.b64decode(res)
	return pickle.loads(res)
