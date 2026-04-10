# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


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
