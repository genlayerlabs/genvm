# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	async def main(self):
		def run():
			raise gl.Rollback('rollback')

		print(await gl.eq_principle_refl(run))
