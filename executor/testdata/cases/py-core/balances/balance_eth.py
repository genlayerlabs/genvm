# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.eth_contract
class EthContract:
	class View:
		pass

	class Write:
		pass


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		print('main self', self.balance)
		print('main At(self)', EthContract(gl.message.contract_account).balance)
		print('=== transfer ===')

		EthContract(gl.message.sender_account).emit_transfer(value=u256(5))
		print('main self', self.balance)
		print('main At(self)', EthContract(gl.message.contract_account).balance)

		print('=== call .view() ===')
		gl.ContractAt(gl.message.contract_account).view().nested()

	@gl.public.view
	def nested(self):
		print('nested self', self.balance)
		print('nested At(self)', EthContract(gl.message.contract_account).balance)
