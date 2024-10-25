# { "Depends": "genlayer-py-std:test" }

import genlayer.std as gl
from genlayer.py.types import *
from genlayer.py.storage import *


@gl.contract
class Contract:
	m: TreeMap[str, u32]

	@gl.public
	def first(self):
		print('first')
		self.m['1'] = u32(12)
		self.m['abc'] = u32(30)

	@gl.public
	def second(self):
		print('second')
		print(list(self.m.items()))
