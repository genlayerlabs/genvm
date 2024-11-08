# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self, addr: Address):
		print('contract from.main')
		try:
			res = gl.ContractAt(addr).view().foo(1, 2).get()
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
