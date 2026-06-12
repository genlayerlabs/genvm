# { "Depends": "py-genlayer:test" }
import genlayer as gl


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.write
	def read_from(self, addr: gl.Address):
		result = gl.contract.get_at(addr).view().get_value()
		print(result)
