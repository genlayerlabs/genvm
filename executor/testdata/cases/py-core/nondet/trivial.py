# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def init(self):
		eval("print('init from eval!')")

		def run():
			print('wow, nondet')
			return 'web page?'

		return gl.eq_principle_strict_eq(run)
