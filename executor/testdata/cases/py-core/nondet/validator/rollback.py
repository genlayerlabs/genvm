# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self):
		def run():
			raise gl.Rollback('rollback')

		print(gl.eq_principle_refl(run).get())
