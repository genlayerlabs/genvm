# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	def foo(self, a, b):
		print('contract to.foo')
		import json

		json.loads = 11  # evil!
		return a + b
