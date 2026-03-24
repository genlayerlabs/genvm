# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	m: gl.TreeMap[str, u32]

	def __init__(self):
		print('first')
		self.m['1'] = 12
		self.m['abc'] = 30

	@gl.public.write
	def second(self):
		print('second')
		print(list(self.m.items()))
