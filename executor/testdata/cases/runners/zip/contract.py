# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def __init__(self):
		print('hello world')
