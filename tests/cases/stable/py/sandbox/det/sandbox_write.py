# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	counter: u8

	def __init__(self):
		print(f'hello world {self.counter}')

		def sb():
			self.counter += 1

		print(f'counter before: {self.counter}')
		gl.vm.spawn_sandbox(sb, allow_write_ops=True)
		print(f'counter after: {self.counter}')
