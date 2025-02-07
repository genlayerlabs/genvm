import contract.lib as lib

from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		lib.foo()
