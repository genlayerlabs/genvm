# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	x: u32
	write_answer: gl.DynArray[u32]

	def __init__(self):
		self.x = 1

	@gl.public.view
	def do_read(self) -> u32:
		return self.x

	@gl.public.write
	def do_test(self):
		# A direct read sees the in-transaction delta; a CallContract (view) reads
		# committed on-chain state and does NOT see the uncommitted write below.
		self.write_answer.append(self.do_read())  # 1
		self.write_answer.append(
			gl.contract.get_at(self.address).view().do_read()
		)  # 1 (committed)
		self.write_answer.append(self.do_read())  # 1

		self.x = 30

		self.write_answer.append(self.do_read())  # 30 (delta)
		self.write_answer.append(
			gl.contract.get_at(self.address).view().do_read()
		)  # 1 (committed)
		self.write_answer.append(self.do_read())  # 30

		print(list(self.write_answer))
