# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self, foo, bar):
		pass

	@gl.public.write
	def foo(self) -> tuple[int, int]:
		return (1, 2)

	@gl.public.write
	def bar(self) -> tuple[int, ...]:
		return (1,)
