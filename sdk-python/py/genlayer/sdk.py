import genlayer.wasi as wasi
import typing

import json
from types import SimpleNamespace
import base64

message = json.loads(wasi.get_message_data(), object_hook=lambda d: SimpleNamespace(**d))
message.account = base64.b64decode(message.account)

def rollback(reason: str) -> typing.NoReturn:
	wasi.rollback(reason)

def run(mod):
	calldata = json.loads(message.calldata)
	meth = getattr(mod, calldata['method'])
	meth(*calldata['args'])
