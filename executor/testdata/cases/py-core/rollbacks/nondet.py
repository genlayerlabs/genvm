# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self):
		try:

			def run():
				gl.rollback_immediate("nah, I won't execute")

			res = gl.eq_principle_refl(run).get()
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
