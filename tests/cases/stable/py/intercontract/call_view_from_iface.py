# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


@gl.contract.interface
class ToIface:
	class View:
		def foo(self, a, b): ...

	class Write:
		pass


class Contract(gl.contract.Contract):
	@gl.public.write
	def main(self, addr: Address):
		print('contract from.main')
		print(ToIface(addr).view().foo(1, 2))
