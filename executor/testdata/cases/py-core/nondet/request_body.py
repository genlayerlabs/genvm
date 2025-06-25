# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			return gl.nondet.web.request(
				f'https://test-server.genlayer.com/body/echo',
				method='POST',
				body=b'\xde\xad\xbe\xef',
			).body

		print(gl.eq_principle.strict_eq(run))
