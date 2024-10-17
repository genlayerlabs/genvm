# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self, addr: gl.Address):
		print('contract from.main')
		print(gl.ContractAt(addr).view().foo(1, 2).get())
