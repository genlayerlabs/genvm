# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl


@gl.contract
class Contract:
	@gl.public
	def __init__(self):
		print('hello world')
