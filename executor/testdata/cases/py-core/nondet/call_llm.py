# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			return (
				gl.nondet.exec_prompt(
					"respond with a single word 'yes' (without quotes) and nothing else"
				)
				.lower()
				.strip()
			)

		print(gl.eq_principle.strict_eq(run))
