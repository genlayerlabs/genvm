# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def foo(self, a, b):
		print('contract to.foo')
		import json

		json.loads = 11  # evil!
		return a + b
