# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self, foo, bar):
		pass

	@gl.public.write
	def foo(self) -> None:
		pass

	@gl.public.write
	def bar(self) -> int:
		return 0
