# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			res = gl.exec_prompt(
				'respond with json object containing single key "result" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes',
				response_format='json',
			)
			if res['result'] < 0 or res['result'] > 100:
				raise Exception(f"invalid result {res["result"]}")
			res['result'] = 42
			return res

		print(gl.eq_principle_strict_eq(run))
