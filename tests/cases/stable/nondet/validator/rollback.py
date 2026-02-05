# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	@gl.public.write
	def main(self):
		def run():
			raise gl.vm.UserError('rollback')

		print(gl.eq_principle.strict_eq(run))
