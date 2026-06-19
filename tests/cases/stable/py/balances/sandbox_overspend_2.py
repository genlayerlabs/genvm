# { "Depends": "py-genlayer:test" }
import genlayer as gl


class Contract(gl.contract.Contract):
	def __init__(self):
		target = gl.message.sender_address

		print(f'balance before={self.balance}')

		def sandbox_fn():
			gl.contract.get_at(target).emit_transfer(value=60)
			return self.balance

		result = gl.vm.spawn_sandbox(sandbox_fn, allow_send_messages=True)
		print(f'sandbox result={result} balance after sandbox={self.balance}')

		try:
			gl.contract.get_at(target).emit_transfer(value=60)
		except Exception as e:
			print(f'transfer failed with error: {e}')

		print(f'balance final={self.balance}')
