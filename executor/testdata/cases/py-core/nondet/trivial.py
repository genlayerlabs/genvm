# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def init(self):
		eval("print('init from eval!')")

		def run():
			print('wow, nondet')
			return 'web page?'

		return gl.eq_principle_refl(run).get()
