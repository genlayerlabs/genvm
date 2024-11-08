# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self):
		gl.rollback_immediate("nah, I won't execute")
