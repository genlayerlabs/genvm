# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self, mode: str):
		def run():
			return gl.get_webpage({'mode': mode}, 'http://127.0.0.1:4242/hello.html').get()

		print(gl.eq_principle_refl(run).get())
