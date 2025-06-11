__all__ = ('evm_contract_interface',)

import typing
import json

from genlayer.py.types import u256
from genlayer.py.evm.generate import contract_generator
from genlayer.py.evm.calldata import MethodEncoder, decode
from . import _lazy_api
import _genlayer_wasi as wasi

import genlayer.gl._internal.gl_call as gl_call


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


evm_contract_interface = contract_generator(
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
