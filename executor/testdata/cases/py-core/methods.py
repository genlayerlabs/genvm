# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as gl
import genlayer.advanced as gla


@gl.contract
class Contract:
	def __init__(self):
		print('init')

	@gl.public
	def pub(self):
		eval("print('init from pub!')")

	@gl.public
	def rback(self):
		gl.rollback_immediate("nah, I won't execute")

	def priv(self):
		eval("print('init from priv!')")

	@gl.public
	def retn(self):
		return {'x': 10}

	@gl.public.view
	def retn(self):
		return {'x': 10}

	@gl.public
	def retn_ser(self):
		return gla.AlreadySerializedResult(b'123')

	@gl.public
	def det_viol(self):
		import json

		gl.wasi.get_webpage(
			json.dumps({'mode': 'text'}), 'http://127.0.0.1:4242/hello.html'
		)
