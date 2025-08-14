# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		gl.advanced.user_error_immediate("nah, I won't execute")
