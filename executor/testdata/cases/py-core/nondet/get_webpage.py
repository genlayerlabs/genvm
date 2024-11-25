# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self, mode: str):
		def run():
			return gl.get_webpage('http://genvm-test/hello.html', mode=mode)  # type: ignore

		print(gl.eq_principle_strict_eq(run))
