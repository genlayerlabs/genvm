# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			raise gl.vm.UserError('rollback')

		print(gl.eq_principle.strict_eq(run))
