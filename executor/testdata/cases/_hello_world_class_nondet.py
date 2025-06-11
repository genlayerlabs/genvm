# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def foo(self):
		gl.vm.run_nondet(lambda: None, lambda x: True)
		print('hello world')
