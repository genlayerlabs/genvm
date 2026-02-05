# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract1(gl.contract.Contract):
	def __init__(self):
		print('hello world')


class Contract2(gl.contract.Contract):
	def __init__(self):
		print('hello world')
