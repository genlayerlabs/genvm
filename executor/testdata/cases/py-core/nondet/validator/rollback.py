# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	async def main(self):
		def run():
			raise gsdk.Rollback('rollback')

		print(await gsdk.eq_principle_refl(run))
