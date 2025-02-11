# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			raise gl.Rollback('rollback')

		print(gl.eq_principle_strict_eq(run))
