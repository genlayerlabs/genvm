import typing

import genlayer.py.calldata as calldata
from genlayer.std._wasi import gl_call as _imp_raw


def _imp(data: calldata.Encodable) -> int:
	return _imp_raw(calldata.encode(data))


def contract_return(data: calldata.Encodable) -> typing.NoReturn:
	_imp({'Return': data})
	assert False


def rollback(data: str) -> typing.NoReturn:
	_imp({'Rollback': data})
	assert False
