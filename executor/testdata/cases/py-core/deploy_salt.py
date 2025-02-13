# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		res = gl.deploy_contract(
			code='not really a contract'.encode('utf-8'), salt_nonce=u256(1)
		)
		print(res)
