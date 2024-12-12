__all__ = ('eth_contract',)

import typing
import json

from genlayer.py.eth.generate import contract_generator
from genlayer.py.eth.calldata import MethodEncoder, decode
from ._internal import lazy_from_fd, _lazy_api
import genlayer.std._wasi as wasi


def _generate_view(name: str, params: list[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode(list(args))
		return lazy_from_fd(
			wasi.eth_call(self.parent.address, calldata),
			lambda x: decode([ret], x),
		)

	return _lazy_api(result_fn)


def _generate_send(name: str, params: list[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode(list(args))
		wasi.eth_send(self.parent.address.as_bytes, calldata)

	return result_fn


eth_contract = contract_generator(_generate_view, _generate_send)
"""
.. code:: python

	@gl.eth_contract
	class Ghost:
		class View:
			pass

		class Write:
			def test(self, x: u256, /) -> None: ...
"""
