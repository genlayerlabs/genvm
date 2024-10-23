# { "Depends": "genlayer-py-std:test" }
import genlayer.std as gl
import genlayer.advanced as gla


@gl.contract
class Contract:
	@gl.public
	def __init__(self):
		gla.run_nondet(lambda: None, lambda x: True)
		print('hello world')
