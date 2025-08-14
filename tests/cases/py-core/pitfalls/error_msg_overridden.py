# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		pass

	@gl.public.write.payable
	def __on_errored_message__(self):
		print('errored but ok')
