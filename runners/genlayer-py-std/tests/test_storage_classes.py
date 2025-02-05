from genlayer.py.storage import *
from genlayer.py.types import *
from genlayer.py.storage._internal.generate import storage, _known_descs


class A:
	x: u32

	def foo(self, other: u32):
		assert self.x == other


class B(A):
	y: u64

	def bar(self, other: u64):
		assert self.y == other


class C(B, A):
	pass


def test_fields():
	X = storage(C)

	x = X()
	x.x = u32(0x01020304)
	x.y = u64(0x05060708090A0B0C)

	assert x.x == 0x01020304
	assert x.y == 0x05060708090A0B0C

	x.foo(u32(0x01020304))
	x.bar(u64(0x05060708090A0B0C))


def test_sizes():
	X = storage(C)
	assert _known_descs[X].size == 12
