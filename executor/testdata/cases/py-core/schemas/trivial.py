# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self, foo, bar):
		pass

	@gl.public.view
	def foo(self):
		pass

	@gl.public.write
	def pos(self, x, y):
		pass

	@gl.public.write
	def kw(self, *, x, y):
		pass

	@gl.public.write
	def mixed(self, a, b, *, x, y):
		pass

	@gl.public.write.payable
	def payable(self):
		pass
