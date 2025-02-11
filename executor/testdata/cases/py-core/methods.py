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
		gl.rollback_immediate("nah, I won't execute")

	def priv(self):
		eval("print('init from priv!')")

	@gl.public.write
	def retn(self):
		return {'x': 10}

	@gl.public.view
	def retn_view(self):
		return {'x': 10}

	@gl.public.write
	def retn_ser(self):
		return gl.advanced.AlreadySerializedResult(b'123')

	@gl.public.write
	def det_viol(self):
		import json

		gl.get_webpage(
			'http://genvm-test/hello.html',
			mode='text',
		)
