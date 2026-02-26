# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	value: u256

	def __init__(self):
		self.value = 42

	@gl.public.view
	def get_value(self) -> int:
		return self.value
