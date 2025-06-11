# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def foo(self, a, b):
		print('contract to.foo')
		gl.advanced.user_error_immediate(f"nah, I won't execute {a + b}")
