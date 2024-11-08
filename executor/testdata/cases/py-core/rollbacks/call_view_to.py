# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def foo(self, a, b):
		print('contract to.foo')
		gl.rollback_immediate(f"nah, I won't execute {a + b}")
