# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self, foo, bar):
		pass

	@gl.public.write
	def foo(self) -> None:
		pass

	@gl.public.write
	def bar(self) -> int:
		return 0
