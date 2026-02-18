# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	@gl.public.write
	def main(self):
		def run():
			gl.vm.user_error_immediate('rollback')

		print(gl.eq_principle.strict_eq(run))
