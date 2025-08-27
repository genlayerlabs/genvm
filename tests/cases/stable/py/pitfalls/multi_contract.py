# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract1(gl.Contract):
	def __init__(self):
		print('hello world')


class Contract2(gl.Contract):
	def __init__(self):
		print('hello world')
