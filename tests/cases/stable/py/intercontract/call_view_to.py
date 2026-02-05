# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.view
	def foo(self, a, b):
		print('contract to.foo')
		import json

		json.loads = 11  # evil!
		return a + b
