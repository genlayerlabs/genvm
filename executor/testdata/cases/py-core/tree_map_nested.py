# { "Depends": "genlayer-py-std:test" }

import genlayer.std as gl

from genlayer.py.types import *
from genlayer.py.storage import *


@gl.contract
class Contract:
	st: TreeMap[Address, TreeMap[Address, u256]]

	@gl.public
	def foo(self):
		first = self.st.get_or_insert_default(Address(b'\x00' * 20))
		print({k.as_hex: dict(v.items()) for k, v in self.st.items()})
		print(dict(first.items()))
		first[Address(b'\x01' * 20)] = u256(13)
		print({k.as_hex: dict(v.items()) for k, v in self.st.items()})
