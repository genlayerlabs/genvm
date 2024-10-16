# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def foo(self, a, b):
		print('contract to.foo')
		gl.rollback_immediate(f"nah, I won't execute {a + b}")
