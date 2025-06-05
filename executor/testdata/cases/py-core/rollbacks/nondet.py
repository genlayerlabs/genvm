# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		try:

			def run():
				gl.advanced.rollback_immediate("nah, I won't execute")

			res = gl.eq_principle.strict_eq(run).get()
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
