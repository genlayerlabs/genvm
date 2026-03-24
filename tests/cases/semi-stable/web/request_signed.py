# { "Depends": "py-genlayer:test" }
import genlayer as gl


class Contract(gl.contract.Contract):
	def __init__(self):
		def run():
			body = b'\xde\xad\xbe\xef'
			res = gl.nondet.web.post(
				'https://test-server.genlayer.com/body/echo-signed',
				body=body,
				headers={},
				sign=True,
			)
			assert res.status == 200, f'expected status 200, got {res.status}'
			assert res.body == body, f'expected body {body!r}, got {res.body!r}'
			return res.body

		print(gl.eq_principle.strict_eq(run))
