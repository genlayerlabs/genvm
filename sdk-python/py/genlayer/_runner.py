import genlayer.wasi as wasi
import genlayer.calldata
import json

def _give_result(res):
	import sys
	if res is None:
		exit(0)
	from genlayer.sdk import AlreadySerializedResult
	if isinstance(res, AlreadySerializedResult):
		wasi.contract_return(res)
	else:
		wasi.contract_return(genlayer.calldata.encode(res))

def run(mod):
	entrypoint: bytes = wasi.get_entrypoint()
	CALL = b'call!'
	NONDET = b'nondet!'
	if entrypoint.startswith(CALL):
		calldata = memoryview(entrypoint)[len(CALL):]
		calldata = genlayer.calldata.decode(calldata)
		meth = getattr(mod, calldata['method'])
		from .sdk import message
		if not message.is_init and not getattr(meth, '__public__', False):
			raise Exception(f"can't call non-public methods")
		res = meth(*calldata['args'])
		_give_result(res)
	elif entrypoint.startswith(NONDET):
		import pickle
		import base64
		res = pickle.loads(entrypoint[len(NONDET):])
		res = res.run()
		wasi.contract_return(pickle.dumps(res))
	else:
		raise Exception(f"unknown entrypoint {entrypoint}")
