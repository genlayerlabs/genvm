# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	async def main(self, addr: gl.Address):
		print('contract from.main')
		try:
			res = await gl.OtherContract(addr).foo(1, 2)
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
