# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract1:
	def __init__(self):
		print('hello world')


@gl.contract
class Contract2:
	def __init__(self):
		print('hello world')
