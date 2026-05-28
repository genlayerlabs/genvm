# { "Depends": "py-genlayer:test" }
import sys

import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self, mb: int):
		def run():
			try:
				res = gl.nondet.web.render(
					f'https://test-server.genlayer.com/static/genvm/fill-ram.html?mb={mb}',
					mode='text',
					wait_after_loaded='5s',
				)
				print(res, file=sys.stderr)
				return 'ok'
			except gl.nondet.NondetException as e:
				return f'error {e.causes} status={e.ctx.get("status")}'

		print(gl.eq_principle.strict_eq(run))
