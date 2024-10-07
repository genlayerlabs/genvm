# { "depends": ["genlayer-py-std:test"] }

import genlayer.sdk as gsdk
from genlayer.py.types import *
from genlayer.py.storage import *


@gsdk.contract
class Contract:
	m: TreeMap[str, u32]

	@gsdk.public
	def first(self):
		print('first')
		self.m['1'] = u32(12)
		self.m['abc'] = u32(30)

	@gsdk.public
	def second(self):
		print('second')
		print(list(self.m.items()))
