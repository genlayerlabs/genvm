# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self, path: str):
		def nondet():
			print(gl.nondet.web.render(path, mode='text'))

		gl.eq_principle.strict_eq(nondet)
