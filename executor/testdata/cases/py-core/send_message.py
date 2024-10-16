# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	def __init__(self):
		gl.ContractAt(gl.Address(b'\x30' * 20)).emit(gas=100).foo(1, 2)
