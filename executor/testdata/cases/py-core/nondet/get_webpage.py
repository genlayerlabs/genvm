# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	async def main(self, mode: str):
		async def run():
			return await gl.get_webpage({'mode': mode}, 'http://127.0.0.1:4242/hello.html')

		print(await gl.eq_principle_refl(run))
