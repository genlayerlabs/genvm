# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self, foo, bar):
		pass

	@gl.public.write
	def foo(self) -> tuple[int, int]:
		return (1, 2)

	@gl.public.write
	def bar(self) -> tuple[int, ...]:
		return (1,)
