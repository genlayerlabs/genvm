# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.view
	def foo(self, a, b):
		print('contract to.foo')
		import json

		json.loads = 11  # evil!
		return a + b
