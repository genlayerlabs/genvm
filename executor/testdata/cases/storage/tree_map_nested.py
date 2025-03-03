# { "Depends": "py-genlayer:test" }

from genlayer import *


class Contract(gl.Contract):
	st: TreeMap[Address, TreeMap[Address, u256]]

	@gl.public.view
	def foo(self):
		first = self.st.get_or_insert_default(Address(b'\x00' * 20))
		print({k.as_hex: dict(v.items()) for k, v in self.st.items()})
		print(dict(first.items()))
		first[Address(b'\x01' * 20)] = u256(13)
		print({k.as_hex: dict(v.items()) for k, v in self.st.items()})
