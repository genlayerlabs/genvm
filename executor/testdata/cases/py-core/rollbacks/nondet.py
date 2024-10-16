# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	async def main(self):
		try:

			def run():
				gl.rollback_immediate("nah, I won't execute")

			res = await gl.eq_principle_refl(run)
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
