# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, mode: str):
		def run():
			return gl.nondet.web.render('http://genvm-test/hello.html', mode=mode)  # type: ignore

		print(gl.eq_principle.strict_eq(run))
