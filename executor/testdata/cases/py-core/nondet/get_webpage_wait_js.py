# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, wait4: str):
		def run():
			return gl.nondet.web.render(
				'http://genvm-test/js-generated.html', mode='text', wait_after_loaded=wait4
			)

		print(gl.eq_principle.strict_eq(run))
