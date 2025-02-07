# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		gl.deploy_contract(code='not really a contract'.encode('utf-8'))
