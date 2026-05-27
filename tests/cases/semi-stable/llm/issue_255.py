# { "Depends": "py-genlayer:test" }
import sys

import genlayer as gl
from genlayer.types import *
from genlayer.nondet import NondetException, _decode_nondet
from genlayer._internal.on_chain import gl_call as _gl_call


class Contract(gl.contract.Contract):
	def __init__(self):
		def run():
			try:
				v = _gl_call.gl_call_generic(
					{
						'ExecPrompt': {
							'prompt': '',
							'response_format': 'text',
							'images': [],
						}
					},
					_decode_nondet,
				).get()
				print(v, file=sys.stderr)
			except NondetException as e:
				print(e.causes)
				print(e.ctx)

		gl.eq_principle.strict_eq(run)
