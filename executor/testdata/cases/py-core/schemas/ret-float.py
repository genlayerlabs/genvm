# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	def __init__(self, foo, bar):
		pass

	@gl.public.write
	def foo(self) -> float:
		return 0.0
