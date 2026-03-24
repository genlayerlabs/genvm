# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		try:

			def run():
				gl.vm.UserError.immediate("nah, I won't execute")

			res = gl.eq_principle.strict_eq(run).get()
		except gl.vm.UserError as r:
			print('handled', r.data)
		else:
			print(res)
			exit(1)
