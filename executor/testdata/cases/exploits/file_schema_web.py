# { "Depends": "py-genlayer:test" }

from genlayer import *


class Contract(gl.Contract):
	def __init__(self, path: str):
		def nondet():
			print(gl.nondet.web.render(path, mode='text'))

		gl.eq_principle.strict_eq(nondet)
