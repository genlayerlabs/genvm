# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	@gl.public.write
	def main(self):
		def run():
			print('SHOULD NOT BE PRINTED')
			return 10

		print(gl.eq_principle.strict_eq(run))
