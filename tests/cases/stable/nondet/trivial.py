# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	@gl.public.write
	def init(self):
		eval("print('init from eval!')")

		def run():
			print('wow, nondet')
			return 'web page?'

		return gl.eq_principle.strict_eq(run)
