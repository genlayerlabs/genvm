# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	counter: u8

	def __init__(self):
		print(f'hello world {self.counter}')
		self.counter += 1

	@gl.public.write
	def foo(self):
		print(f'hello world {self.counter}')
		self.counter += 1
