# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.write
	def foo(self, a, b):
		print('contract to.foo')
		gl.vm.UserError.immediate({'error': 'some_error', 'code': a + b})
