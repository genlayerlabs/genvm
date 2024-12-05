# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract_interface
class ToIface:
	class View:
		def foo(self, a, b): ...

	class Write:
		pass


@gl.contract
class Contract:
	@gl.public.write
	def main(self, addr: Address):
		print('contract from.main')
		print(ToIface(addr).view().foo(1, 2))
