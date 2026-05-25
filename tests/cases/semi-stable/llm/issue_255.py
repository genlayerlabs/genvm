# { "Depends": "py-genlayer:test" }
import sys

import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		def run():
			try:
				v = gl.nondet.exec_prompt(
					'',
					response_format='text',
				)
				print(v, file=sys.stderr)
			except gl.nondet.NondetException as e:
				print(e.args)

		gl.eq_principle.strict_eq(run)
