# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self, addr: Address):
		print('contract from.main')
		print(gl.ContractAt(addr).view().foo(1, 2))
