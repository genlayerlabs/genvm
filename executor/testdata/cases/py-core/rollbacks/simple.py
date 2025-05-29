# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		gl.advanced.rollback_immediate("nah, I won't execute")
