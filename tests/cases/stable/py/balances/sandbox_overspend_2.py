# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		target = gl.message.sender_address

		print(f'balance before={self.balance}')

		def sandbox_fn():
			gl.get_contract_at(target).emit_transfer(value=u256(60))
			return self.balance

		result = gl.vm.spawn_sandbox(sandbox_fn, allow_write_ops=True)
		print(f'sandbox result={result} balance after sandbox={self.balance}')

		try:
			gl.get_contract_at(target).emit_transfer(value=u256(60))
		except Exception as e:
			print(f'transfer failed with error: {e}')

		print(f'balance final={self.balance}')
