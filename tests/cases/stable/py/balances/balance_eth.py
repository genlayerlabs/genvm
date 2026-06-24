# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


@gl.evm.contract_interface
class EthContract:
	class View:
		pass

	class Write:
		pass


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.write
	def main(self):
		print('main self', self.balance)
		print('main At(self)', EthContract(gl.message.contract_address).balance)
		print('=== transfer ===')

		EthContract(gl.message.sender_address).emit_transfer(value=5)
		print('main self', self.balance)
		print('main At(self)', EthContract(gl.message.contract_address).balance)

		print('=== call .view() ===')
		gl.contract.get_at(gl.message.contract_address).view().nested()

	@gl.public.view
	def nested(self):
		print('nested self', self.balance)
		print('nested At(self)', EthContract(gl.message.contract_address).balance)
