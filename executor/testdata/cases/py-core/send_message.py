# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gsdk


@gsdk.contract
class Contract:
	def __init__(self):
		gsdk.OtherContract(gsdk.Address(b'\x30' * 32)).foo.send(1, 2, gas=100)
