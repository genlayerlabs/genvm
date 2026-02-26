# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		pass

	@gl.public.write
	def read_from(self, addr: Address):
		result = gl.get_contract_at(addr).view().get_value()
		print(result)
