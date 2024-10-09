# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as std

import typing


@std.contract
class Contract:
	def __init__(self):
		pass

	@std.public
	def foo(self, a1: int, a2: None, a3: bool, a4: str, a5: bytes, a6: std.Address):
		pass

	@std.public
	def erased(self, a1: list, a2: dict, a3: typing.Any):
		pass
