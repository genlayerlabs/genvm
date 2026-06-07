# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


# Regression guard (genvm fee-allocation item #8): EthSend (external) variant.
#
# An empty message-fee-allocation tree must make an external (eth) send hard-trap
# (genlayer_sdk.rs EthSend path: no matching node -> oom_trap fees().external()),
# never silently emit nothing. The balance check (value=30 <= 100) passes first,
# so the trap comes from the allocation lookup, not from balance.
#
# Paired positive case: ../intercontract/send_message_eth.py (default tree ->
# EthSend emission succeeds).
@gl.evm.contract_interface
class Ghost:
	class View:
		pass

	class Write:
		def test(self, x: u256, /) -> None: ...


class Contract(gl.contract.Contract):
	def __init__(self):
		print(self.balance)
		Ghost(Address(b'\x30' * 20)).emit(value=30).test(10)
