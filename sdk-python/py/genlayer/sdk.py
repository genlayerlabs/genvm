import genlayer.wasi as wasi
import typing

import json
from types import SimpleNamespace
import base64

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

def run(mod):
	entrypoint = message_raw["entrypoint"]
	if 'Call' in entrypoint:
		calldata = json.loads(entrypoint['Call'])
		meth = getattr(mod, calldata['method'])
		res = meth(*calldata['args'])
		if res is None:
			exit(0)
		elif isinstance(res, AlreadySerializedResult):
			wasi.contract_return(res)
		else:
			wasi.contract_return(json.dumps(res))
	else:
		raise Exception(f"unknown entrypoint {entrypoint}")
