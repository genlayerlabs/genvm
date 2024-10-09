# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as std


@std.contract
class Contract:
	def __init__(self, foo, bar):
		pass

	@std.public
	def foo(self) -> None:
		pass

	@std.public
	def bar(self) -> int:
		return 0
