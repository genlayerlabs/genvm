__all__ = ('eth_contract',)

import typing
import json

from genlayer.py.types import u256
from genlayer.py.eth.generate import contract_generator
from genlayer.py.eth.calldata import MethodEncoder, decode
from ._internal import lazy_from_fd, _lazy_api
import genlayer.std._wasi as wasi

from genlayer.py.eth.generate import transaction_data_kw_args_serialize


def _generate_view(name: str, params: tuple[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode_call(args)
		return lazy_from_fd(
			wasi.eth_call(self.parent.address, calldata),
			lambda x: decode(ret, x),
		)

	return _lazy_api(result_fn)


def _generate_send(name: str, params: tuple[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode_call(args)
		assert len(self._proxy_args) == 1
		assert len(self._proxy_kwargs) == 0
		data = json.dumps(self._proxy_args[0])
		wasi.eth_send(self._proxy_parent.address.as_bytes, calldata, data)

	return result_fn


eth_contract = contract_generator(
	_generate_view,
	_generate_send,
	lambda p: u256(wasi.get_balance(p.address.as_bytes)),
	lambda p, d: wasi.eth_send(
		p.address.as_bytes, b'', transaction_data_kw_args_serialize(dict(d))
	),
)
"""
Decorator that is used to declare eth contract interface

.. code:: python

	@gl.eth_contract
	class Ghost:
		class View:
			pass

		class Write:
			def test(self, x: u256, /) -> None: ...
"""
