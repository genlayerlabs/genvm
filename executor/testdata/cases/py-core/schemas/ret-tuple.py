# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	def __init__(self, foo, bar):
		pass

	@gl.public.write
	def foo(self) -> tuple[int, int]:
		return (1, 2)

	@gl.public.write
	def bar(self) -> tuple[int, ...]:
		return (1,)
