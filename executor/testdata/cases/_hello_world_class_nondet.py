# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def __init__(self):
		gl.advanced.run_nondet(lambda: None, lambda x: True)
		print('hello world')
