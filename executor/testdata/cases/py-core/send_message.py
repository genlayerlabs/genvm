# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		gl.ContractAt(gl.Address(b'\x30' * 20)).emit().foo(1, 2)
