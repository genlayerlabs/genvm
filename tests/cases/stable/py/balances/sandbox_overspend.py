# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		balance_before = self.balance
		target = gl.message.sender_address

		# First transfer: spend 60 out of 100
		gl.get_contract_at(target).emit_transfer(value=u256(60))
		balance_after_first = self.balance
		print(f'balance before={balance_before} after_first_send={balance_after_first}')

		# Now spawn a sandbox with write ops allowed.
		# BUG: the sandbox gets a fresh messages_decremented=0,
		# so it thinks the full balance is still available.
		def sandbox_fn():
			# Second transfer: spend another 60 inside sandbox
			# This should fail (total 120 > 100) but the sandbox
			# doesn't know about the parent's 60 already spent.
			gl.get_contract_at(target).emit_transfer(value=u256(60))
			return self.balance

		result = gl.vm.spawn_sandbox(sandbox_fn, allow_write_ops=True)
		print(f'sandbox result={result}')
		print(f'balance final={self.balance}')
		# If we reach here with both transfers accepted,
		# total spent is 120 but balance was only 100 => overspend bug
