# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	@gl.public.write
	def main(self, on: str):
		try:
			gl.contract.get_at(gl.Address(b'\x30' * 20)).emit(on=on).foo(1, 2)
		except SystemError as e:
			print(e)
