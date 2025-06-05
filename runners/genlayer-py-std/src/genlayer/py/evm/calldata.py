__all__ = (
	'signature_of',
	'type_name_of',
	'selector_of',
	'MethodEncoder',
	'encode',
	'decode',
)

from ..keccak import Keccak256

import typing

from genlayer.py.types import *
import genlayer.py._internal.reflect as reflect

from ._internal.builder import build
from ._internal.codecs import EncodeState, DecoderState
from .support import *


def type_name_of(t: type) -> str:
	return build(t).name


def signature_of[*T](name: str, params: typing.Type[tuple[*T]]) -> str:
	"""
	calculates signature that is used for making method selector
	"""
	return name + build(params).name


def selector_of[*T](name: str, params: typing.Type[tuple[*T]]) -> bytes:
	return Keccak256(signature_of(name, params).encode('utf-8')).digest()[:4]


def encode[T](params: typing.Type[T], args: T) -> bytes:
	encoder = build(params)
	state = EncodeState(0, bytearray(), [])
	encoder.encode(state, args)
	state.run_tails()
	return bytes(state.result)


def decode[T](expected: typing.Type[T], encoded: collections.abc.Buffer) -> T:
	encoder = build(expected)
	state = DecoderState(memoryview(encoded), 0, 0)
	return encoder.decode(state)


class MethodEncoder:
	__slots__ = ('_encoder', '_selector', '_decoder')

	def __init__(self, name: str, params: tuple[typing.Any, ...], ret: type):
		self._encoder = build(tuple[InplaceTuple, *params])
		if reflect.is_none_type(ret):
			self._decoder = None
		else:
			self._decoder = build(ret)

		selector_str = name + self._encoder.name
		self._selector = Keccak256(selector_str.encode('utf-8')).digest()[:4]

	def encode_call(self, args: tuple[typing.Any, ...]) -> bytes:
		state = EncodeState(4, bytearray(self._selector), [])
		self._encoder.encode(state, args)
		state.run_tails()
		return bytes(state.result)

	def decode_ret(self, data: collections.abc.Buffer) -> typing.Any:
		state = DecoderState(memoryview(data), 0, 0)
		if self._decoder is None:
			return None
		return self._decoder.decode(state)
