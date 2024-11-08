# { "Depends": "py-genlayer:test" }

from genlayer import *


@gl.contract
class Contract:
	m: TreeMap[str, u32]

	@gl.public.write
	def first(self):
		print('first')
		self.m['1'] = u32(12)
		self.m['abc'] = u32(30)

	@gl.public.write
	def second(self):
		print('second')
		print(list(self.m.items()))
