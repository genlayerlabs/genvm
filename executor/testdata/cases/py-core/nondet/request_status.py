# { "Depends": "py-genlayer:test" }
from genlayer import *

import sys


class Contract(gl.Contract):
	@gl.public.write
	def main(self, status: int):
		def run():
			res = gl.nondet.web.request(
				f'https://test-server.genlayer.com/status/{status}', method='GET'
			)
			print(res, file=sys.stderr)
			return res.status

		print(gl.eq_principle.strict_eq(run))
