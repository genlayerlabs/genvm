# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		print('hello world')
