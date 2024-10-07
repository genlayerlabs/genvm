import genlayer.wasi as wasi
import genlayer.py.calldata
import typing
from genlayer.py.types import Rollback
from ._private import _call_user_fn


def _give_result(res_fn: typing.Callable[[], typing.Any]):
	try:
		res = _call_user_fn(res_fn)
	except Rollback as r:
		wasi.rollback(r.msg)
	from genlayer.sdk import AlreadySerializedResult

	if isinstance(res, AlreadySerializedResult):
		wasi.contract_return(res)
	else:
		wasi.contract_return(genlayer.py.calldata.encode(res))


def run(contract: type):
	entrypoint: bytes = wasi.get_entrypoint()
	mem = memoryview(entrypoint)
	CALL = b'call!'
	NONDET = b'nondet!'
	if entrypoint.startswith(CALL):
		mem = mem[len(CALL) :]
		calldata = genlayer.py.calldata.decode(mem)
		meth = getattr(contract, calldata['method'])
		from .sdk import message

		if not message.is_init and not getattr(meth, '__public__', False):
			raise Exception(f"can't call non-public methods")
		from .storage import STORAGE_MAN, ROOT_STORAGE_ADDRESS

		top_slot = STORAGE_MAN.get_store_slot(ROOT_STORAGE_ADDRESS)
		contract_instance = contract.__view_at__(top_slot, 0)
		_give_result(lambda: meth(contract_instance, *calldata['args']))
	elif entrypoint.startswith(NONDET):
		mem = mem[len(NONDET) :]
		# fetch leaders result length
		le = int.from_bytes(mem[:4], 'little')
		mem = mem[4:]

		leaders_res_mem = mem[:le]
		mem = mem[le:]
		import cloudpickle

		runner = cloudpickle.loads(mem)
		if le == 0:
			_give_result(runner)
		else:
			from ._private import _decode_sub_vm_result_retn

			leaders_res = _decode_sub_vm_result_retn(leaders_res_mem)
			_give_result(lambda: runner(leaders_res))
	else:
		raise Exception(f'unknown entrypoint {entrypoint}')
