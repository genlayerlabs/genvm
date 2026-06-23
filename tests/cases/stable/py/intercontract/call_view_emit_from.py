# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.write
	def main(self, addr: Address):
		# read-only CallContract into B; B tries to emit an event and send a
		# message inside it. Both must fail because the child is a static call.
		try:
			print('A got', gl.contract.get_at(addr).view().ping())
		except Exception as e:
			print(f'A call failed: {type(e).__name__}')
