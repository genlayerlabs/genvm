# { "Depends": "py-genlayer:test" }
import genlayer as gl


class Contract(gl.contract.Contract):
	value: gl.u256

	def __init__(self):
		self.value = 42

	@gl.public.view
	def get_value(self) -> int:
		return self.value
