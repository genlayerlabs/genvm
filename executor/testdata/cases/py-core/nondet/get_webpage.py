# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	async def main(self, mode: str):
		async def run():
			return await gsdk.get_webpage({'mode': mode}, 'http://127.0.0.1:4242/hello.html')

		print(await gsdk.eq_principle_refl(run))
