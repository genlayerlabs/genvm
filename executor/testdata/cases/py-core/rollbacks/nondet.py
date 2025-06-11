# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		try:

			def run():
				gl.advanced.user_error_immediate("nah, I won't execute")

			res = gl.eq_principle.strict_eq(run).get()
		except gl.vm.UserError as r:
			print('handled', r.message)
		else:
			print(res)
			exit(1)
