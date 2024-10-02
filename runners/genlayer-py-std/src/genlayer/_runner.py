import genlayer.wasi as wasi
import genlayer.py.calldata
import typing
from genlayer.py.types import Rollback

def _give_result(res_fn: typing.Callable[[], typing.Any]):
	try:
		res = res_fn()
	except Rollback as r:
		wasi.rollback(r.msg)
	if hasattr(res, '__await__'):
		try:
			res.send(None)
		except StopIteration as si:
			res = si.value
		except Rollback as r:
			wasi.rollback(r.msg)
		else:
			raise Exception(f"no send for awaitable {res}")
	if res is None:
		exit(0)
	from genlayer.sdk import AlreadySerializedResult
	if isinstance(res, AlreadySerializedResult):
		wasi.contract_return(res)
	else:
		wasi.contract_return(genlayer.py.calldata.encode(res))

def run(mod):
	entrypoint: bytes = wasi.get_entrypoint()
	CALL = b'call!'
	NONDET = b'nondet!'
	if entrypoint.startswith(CALL):
		calldata = memoryview(entrypoint)[len(CALL):]
		calldata = genlayer.py.calldata.decode(calldata)
		meth = getattr(mod, calldata['method'])
		from .sdk import message
		if not message.is_init and not getattr(meth, '__public__', False):
			raise Exception(f"can't call non-public methods")
		_give_result(lambda: meth(*calldata['args']))
	elif entrypoint.startswith(NONDET):
		import pickle
		runner = pickle.loads(entrypoint[len(NONDET):])
		_give_result(lambda: runner.run())
	else:
		raise Exception(f"unknown entrypoint {entrypoint}")
