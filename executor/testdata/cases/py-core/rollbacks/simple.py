# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self):
		gl.rollback_immediate("nah, I won't execute")
