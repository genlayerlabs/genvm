# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		res = gl.contract.deploy(code='not really a contract'.encode('utf-8'), salt_nonce=1)
		print(res)
