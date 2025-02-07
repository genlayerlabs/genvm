# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def foo(self):
		gl.advanced.run_nondet(lambda: None, lambda x: True)
		print('hello world')
