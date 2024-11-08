# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract1:
	def __init__(self):
		print('hello world')


@gl.contract
class Contract2:
	def __init__(self):
		print('hello world')
