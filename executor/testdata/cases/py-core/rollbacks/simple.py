# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	def main(self):
		gsdk.rollback_immediate("nah, I won't execute")
