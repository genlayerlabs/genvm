# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		print('init')

	@gl.public.write
	def pub(self):
		eval("print('init from pub!')")

	@gl.public.write
	def rback(self):
		gl.advanced.user_error_immediate("nah, I won't execute")

	def priv(self):
		eval("print('init from priv!')")

	@gl.public.write
	def retn(self):
		return {'x': 10}

	@gl.public.view
	def retn_view(self):
		return {'x': 10}

	@gl.public.write
	def det_viol(self):
		gl.nondet.web.render(
			'http://genvm-test/hello.html',
			mode='text',
		)
