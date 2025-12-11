# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	counter: u8

	@gl.public.write
	def foo(self):
		print(f'hello world {self.counter}')
		self.counter += 1
