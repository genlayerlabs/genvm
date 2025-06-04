# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, status: int):
		def run():
			return gl.request(f'https://httpstat.us/{status}', method='GET').status

		print(gl.eq_principle_strict_eq(run))
