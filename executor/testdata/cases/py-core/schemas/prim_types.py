# { "Depends": "py-genlayer:test" }

from genlayer import *

import typing


@gl.contract
class Contract:
	def __init__(self):
		pass

	@gl.public.write
	def foo(self, a1: int, a2: None, a3: bool, a4: str, a5: bytes, a6: Address):
		pass

	@gl.public.write
	def erased(self, a1: list, a2: dict, a3: typing.Any):
		pass
