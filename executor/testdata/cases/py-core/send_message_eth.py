# { "Depends": "py-genlayer:test" }
from genlayer import *


@gl.evm.contract_interface
class Ghost:
	class View:
		pass

	class Write:
		def test(self, x: u256, /) -> None: ...


class Contract(gl.Contract):
	def __init__(self):
		Ghost(Address(b'\x30' * 20)).emit().test(u256(10))
