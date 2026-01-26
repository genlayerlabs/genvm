# { "Depends": "py-genlayer:test" }
from genlayer import *

import genlayer.gl._internal.gl_call as gl_call
from genlayer.gl.nondet.web import Response
from genlayer.gl.nondet import _decode_nondet


class Contract(gl.Contract):
	def __init__(self):
		def run():
			body = b'\xde\xad\xbe\xef'
			res = gl_call.gl_call_generic(
				{
					'WebRequest': {
						'url': 'https://test-server.genlayer.com/body/echo-signed',
						'method': 'POST',
						'body': body,
						'headers': {},
						'sign': True,
					}
				},
				lambda x: Response(**(_decode_nondet(x)['response'])),
			).get()
			assert res.status == 200, f'expected status 200, got {res.status}'
			assert res.body == body, f'expected body {body!r}, got {res.body!r}'
			return res.body

		print(gl.eq_principle.strict_eq(run))
