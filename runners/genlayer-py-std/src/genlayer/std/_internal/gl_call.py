import typing
import os

import genlayer.py.calldata as calldata
import collections.abc
from genlayer.py.types import Lazy
from genlayer.std._wasi import gl_call as _imp_raw


def _imp(data: calldata.Encodable) -> int:
	return _imp_raw(calldata.encode(data))


def contract_return(data: calldata.Encodable) -> typing.NoReturn:
	_imp({'Return': data})
	assert False


def rollback(data: str) -> typing.NoReturn:
	_imp({'Rollback': data})
	assert False


def gl_call_generic[T](
	data: calldata.Encodable, after: typing.Callable[[collections.abc.Buffer], T]
) -> Lazy[T]:
	fd = _imp_raw(calldata.encode(data))
	if fd == 2**32 - 1:
		return None  # type: ignore

	def run():
		with os.fdopen(fd, 'rb') as f:
			code = f.read(1)
			rest = f.read()
		if code != b'\x00':
			raise Exception('recoverable error', rest.decode('utf-8'))
		return after(rest)

	return Lazy(run)
