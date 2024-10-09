# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	async def main(self):
		try:

			def run():
				gsdk.rollback_immediate("nah, I won't execute")

			res = await gsdk.eq_principle_refl(run)
		except gsdk.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
