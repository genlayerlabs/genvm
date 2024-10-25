# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def main(self, addr: gl.Address):
		print('contract from.main')
		try:
			res = gl.ContractAt(addr).view().foo(1, 2).get()
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
