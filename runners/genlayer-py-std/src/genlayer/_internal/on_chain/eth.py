__all__ = ('evm_contract_interface',)

import typing
import json

from genlayer.types import Address, u256
from genlayer.evm.generate import TransactionDataKwArgs, contract_generator
from genlayer.evm.calldata import MethodEncoder, decode
from genlayer._internal import _lazy_api
import _genlayer_wasi as wasi

import genlayer._internal.on_chain.gl_call as gl_call


def _generate_view(name: str, params: tuple[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode_call(args)
		return gl_call.gl_call_generic(
			{
				'EthCall': {
					'address': self._proxy_parent.address,
					'calldata': calldata,
				}
			},
			lambda x: decode(ret, x),
		)

	return _lazy_api(result_fn)


def perform_send(address: Address, calldata: bytes, value: u256) -> None:
	gl_call.gl_call_generic(
		{
			'EthSend': {
				'address': address,
				'calldata': calldata,
				'value': value,
			}
		},
		lambda _x: None,
	).get()


def _generate_send(name: str, params: tuple[type], ret: type) -> typing.Any:
	encoder = MethodEncoder(name, params, ret)

	def result_fn(self, *args):
		calldata = encoder.encode_call(args)
		if len(self._proxy_args) != 1:
			raise TypeError(f'expected exactly 1 proxy arg, got {len(self._proxy_args)}')
		if len(self._proxy_kwargs) != 0:
			raise TypeError(
				f'expected no proxy kwargs, got {sorted(self._proxy_kwargs.keys())}'
			)
		a = typing.cast(TransactionDataKwArgs, self._proxy_args[0])
		perform_send(self._proxy_parent.address, calldata, a.get('value', 0))

	return result_fn


evm_contract_interface = contract_generator(
	_generate_view,
	_generate_send,
	lambda p: wasi.get_balance(p.address.as_bytes),
	lambda p, d: gl_call.gl_call_generic(
		{'EthSend': {'address': p.address, 'calldata': b'', 'value': d.get('value', 0)}},
		lambda _x: None,
	).get(),
)

evm_contract_interface.__doc__ = """
Decorator that is used to declare eth contract interface

.. code:: python

	@gl.eth_contract
	class Ghost:
		class View:
			pass

		class Write:
			def test(self, x: u256, /) -> None: ...
"""
