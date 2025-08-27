# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, mode: str):
		def run():
			return gl.nondet.web.render(
				'https://test-server.genlayer.com/static/genvm/hello.html', mode=mode
			)  # type: ignore

		print(gl.eq_principle.strict_eq(run))
