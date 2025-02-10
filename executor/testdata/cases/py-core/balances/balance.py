# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		print('main self', self.balance)
		print('main At(self)', gl.ContractAt(gl.message.contract_address).balance)
		print('=== transfer ===')
		gl.ContractAt(gl.message.sender_address).emit_transfer(value=u256(5))
		print('main self', self.balance)
		print('main At(self)', gl.ContractAt(gl.message.contract_address).balance)

		print('=== call .view() ===')
		gl.ContractAt(gl.message.contract_address).view().nested()

	@gl.public.view
	def nested(self):
		print('nested self', self.balance)
		print('nested At(self)', gl.ContractAt(gl.message.contract_address).balance)
