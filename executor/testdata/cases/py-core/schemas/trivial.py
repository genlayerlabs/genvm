# { "depends": ["genlayer-py-std:test"] }
import genlayer.std as std


@std.contract
class Contract:
	def __init__(self, foo, bar):
		pass

	@std.public
	def foo(self):
		pass

	@std.public
	def pos(self, x, y):
		pass

	@std.public
	def kw(self, *, x, y):
		pass

	@std.public
	def mixed(self, a, b, *, x, y):
		pass
