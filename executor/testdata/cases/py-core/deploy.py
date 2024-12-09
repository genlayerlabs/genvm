# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.contract
class Contract:
	def __init__(self):
		gl.deploy_contract(code='not really a contract'.encode('utf-8'), gas=300)
