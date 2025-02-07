# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, mode: str):
		def run():
			return gl.get_webpage('http://genvm-test/hello.html', mode=mode)  # type: ignore

		print(gl.eq_principle_strict_eq(run))
