# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			print('SHOULD NOT BE PRINTED')
			return 10

		print(gl.eq_principle_strict_eq(run))
