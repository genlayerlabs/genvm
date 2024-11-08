# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self):
		def run():
			raise gl.Rollback('rollback')

		print(gl.eq_principle_strict_eq(run))
