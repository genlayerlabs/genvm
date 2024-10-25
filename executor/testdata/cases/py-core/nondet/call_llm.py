# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl
import json


@gl.contract
class Contract:
	@gl.public
	def main(self):
		def run():
			return gl.exec_prompt({}, "print 'yes' (without quotes) and nothing else").get()

		print(gl.eq_principle_refl(run).get())
