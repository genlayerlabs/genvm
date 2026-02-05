# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	st: gl.TreeMap[Address, gl.TreeMap[Address, u256]]

	def __init__(self):
		first = self.st.get_or_insert_default(Address(b'\x00' * 20))
		print({k.as_hex: dict(v.items()) for k, v in self.st.items()})
		print(dict(first.items()))
		first[Address(b'\x01' * 20)] = u256(13)
		print({k.as_hex: dict(v.items()) for k, v in self.st.items()})
