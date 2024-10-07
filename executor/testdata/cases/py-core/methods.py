# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk


@gsdk.contract
class Contract:
	def __init__(self):
		print('init')

	@gsdk.public
	def pub(self):
		eval("print('init from pub!')")

	@gsdk.public
	def rback(self):
		gsdk.rollback_immediate("nah, I won't execute")

	def priv(self):
		eval("print('init from priv!')")

	@gsdk.public
	def retn(self):
		return {'x': 10}

	@gsdk.public
	def retn_ser(self):
		return gsdk.AlreadySerializedResult(b'123')

	@gsdk.public
	def det_viol(self):
		import json

		gsdk.wasi.get_webpage(
			json.dumps({'mode': 'text'}), 'http://127.0.0.1:4242/hello.html'
		)
