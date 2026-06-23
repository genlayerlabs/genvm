# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Pinged(gl.chain.Event):
	def __init__(self, who: str, /, **blob): ...


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.view
	def ping(self) -> str:
		# inside a CallContract child VM (read-only / static call)
		try:
			Pinged('b', note='emitted during CallContract').emit()
			print('B emit: ok')
		except Exception as e:
			print(f'B emit forbidden: {type(e).__name__}')
		try:
			gl.contract.get_at(gl.Address(b'\x30' * 20)).emit().foo(1, 2)
			print('B send: ok')
		except Exception as e:
			print(f'B send forbidden: {type(e).__name__}')
		return 'pong'
