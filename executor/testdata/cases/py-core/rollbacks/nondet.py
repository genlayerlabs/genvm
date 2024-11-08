# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	@gl.public.write
	def main(self):
		try:

			def run():
				gl.rollback_immediate("nah, I won't execute")

			res = gl.eq_principle_strict_eq(run).get()
		except gl.Rollback as r:
			print('handled', r.msg)
		else:
			print(res)
			exit(1)
