# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self):
		def run():
			print('SHOULD NOT BE PRINTED')
			return 10

		print(gl.eq_principle_strict_eq(run))
