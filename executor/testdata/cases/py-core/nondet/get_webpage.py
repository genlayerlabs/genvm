# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self, mode: str):
		def run():
			return gl.get_webpage('http://127.0.0.1:4242/hello.html', mode=mode)

		print(gl.eq_principle_strict_eq(run))
