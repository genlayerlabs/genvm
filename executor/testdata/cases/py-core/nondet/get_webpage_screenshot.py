# { "Depends": "py-genlayer:test" }
from genlayer import *

import sys


class Contract(gl.Contract):
	@gl.public.write
	def main(self, mode: str):
		def run():
			img = gl.get_webpage('http://genvm-test/hello.html', mode='screenshot')

			res = gl.exec_prompt(
				'what image says? respond only with its contents', images=[img]
			)

			return ''.join(c for c in res.strip().lower() if c.isalpha())

		res = gl.eq_principle_strict_eq(run)
		print(res, file=sys.stderr)
		print('helloworld' in res)
