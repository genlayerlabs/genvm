# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, on: str):
		try:
			gl.ContractAt(gl.Address(b'\x30' * 20)).emit(on=on).foo(1, 2)
		except SystemError as e:
			print(e)
