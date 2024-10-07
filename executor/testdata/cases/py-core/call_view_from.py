# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	async def main(self, addr: gsdk.Address):
		print('contract from.main')
		print(await gsdk.OtherContract(addr).foo(1, 2))
