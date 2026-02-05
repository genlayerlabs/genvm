# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.write
	def main(self, addr: Address):
		print('contract from.main')
		print(gl.contract.get_at(addr).view().foo(1, 2))
