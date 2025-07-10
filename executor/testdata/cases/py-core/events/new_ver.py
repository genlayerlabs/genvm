# v999.2.0
# { "Depends": "py-genlayer:test" }

from genlayer import *


class Ev(gl.Event):
	def __init__(self, a, b, /, **blob): ...


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		try:
			Ev(1, 2).emit()
		except Exception as e:
			print(e)
