# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl
import json


@gl.contract
class Contract:
	@gl.public
	async def main(self):
		async def run():
			return await gl.exec_prompt({}, "print 'yes' (without quotes) and nothing else")

		print(await gl.eq_principle_refl(run))
