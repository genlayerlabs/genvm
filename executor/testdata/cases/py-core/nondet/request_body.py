# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			return gl.nondet.web.request(
				f'https://httpbin.org/bytes/16?seed=0', method='GET'
			).body

		print(gl.eq_principle.strict_eq(run))
