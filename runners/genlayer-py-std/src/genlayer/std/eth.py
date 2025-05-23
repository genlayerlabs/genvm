__all__ = ('eth_contract',)

import typing
import json

from genlayer.py.types import u256
from genlayer.py.eth.generate import contract_generator
from genlayer.py.eth.calldata import MethodEncoder, decode
from ._internal import _lazy_api
import genlayer.std._wasi as wasi

import genlayer.std._internal.gl_call as gl_call

from genlayer.py.eth.generate import transaction_data_kw_args_serialize


def _generate_view(name: str, params: tuple[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode_call(args)
		return gl_call.gl_call_generic(
			{
				'EthCall': {
					'address': self.parent.address,
					'calldata': calldata,
				}
			},
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
		gl_call.gl_call_generic(
			{
				'EthSend': {
					'address': self._proxy_parent.address,
					'calldata': calldata,
					'value': self._proxy_kwargs.get('value', 0),
				}
			},
			lambda _x: None,
		).get()

	return result_fn


eth_contract = contract_generator(
	_generate_view,
	_generate_send,
	lambda p: u256(wasi.get_balance(p.address.as_bytes)),
	lambda p, d: gl_call.gl_call_generic(
		{'EthSend': {'address': p.address, 'calldata': b'', 'value': d.get('value', 0)}},
		lambda _x: None,
	).get(),
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
