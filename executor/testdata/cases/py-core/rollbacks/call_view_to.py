# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	def foo(self, a, b):
		print('contract to.foo')
		gsdk.rollback_immediate(f"nah, I won't execute {a + b}")
