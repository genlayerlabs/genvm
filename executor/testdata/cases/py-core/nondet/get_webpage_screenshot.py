# { "Depends": "py-genlayer:test" }
from genlayer import *

import sys


class Contract(gl.Contract):
	@gl.public.write
	def main(self, mode: str):
		def run():
			img = gl.nondet.web.render(
				'https://test-server.genlayer.com/static/genvm/hello.html', mode='screenshot'
			)

			res = gl.nondet.exec_prompt(
				'what image says? respond only with its contents', images=[img]
			)

			return ''.join(c for c in res.strip().lower() if c.isalpha())

		res = gl.eq_principle.strict_eq(run)
		print(res, file=sys.stderr)
		print('helloworld' in res)
