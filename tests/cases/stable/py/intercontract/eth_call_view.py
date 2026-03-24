# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


@gl.evm.contract_interface
class Ghost:
	class View:
		def test(self, x: u256, /) -> u256: ...

	class Write:
		pass


class Contract(gl.contract.Contract):
	def __init__(self):
		try:
			Ghost(Address(b'\x30' * 20)).view().test(10)
		except BaseException as e:
			print(f'expected error: {e}')
