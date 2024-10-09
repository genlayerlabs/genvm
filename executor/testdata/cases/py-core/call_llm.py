# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gsdk
import json


@gsdk.contract
class Contract:
	@gsdk.public
	async def main(self):
		async def run():
			return await gsdk.call_llm({}, "print 'yes' (without quotes) and nothing else")

		print(await gsdk.eq_principle_refl(run))
