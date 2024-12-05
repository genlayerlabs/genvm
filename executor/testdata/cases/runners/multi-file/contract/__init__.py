import contract.lib as lib

from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self):
		lib.foo()
