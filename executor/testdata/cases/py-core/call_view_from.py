# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self, addr: gl.Address):
		print('contract from.main')
		print(gl.OtherContract(addr).foo(1, 2).get())
