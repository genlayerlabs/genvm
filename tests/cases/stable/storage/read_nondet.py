# { "Depends": "py-genlayer:latest" }

from genlayer import *


class Storage(gl.Contract):
	a: str

	def __init__(self):
		self.a = '123'

		def f():
			print(self.a)

		gl.eq_principle.strict_eq(f)
		self.a = '456'
		gl.eq_principle.strict_eq(f)
