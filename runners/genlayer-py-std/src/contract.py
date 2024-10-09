# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gsdk


@gsdk.contract
class Contract:
	@gsdk.public
	def __init__(self):
		print('hello world')
