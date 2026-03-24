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
		Ghost(Address(b'\x30' * 20)).emit().test(10)
