# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl


@gl.contract
class Contract:
	def __init__(self):
		gl.OtherContract(gl.Address(b'\x30' * 20)).foo.send(1, 2, gas=100)
