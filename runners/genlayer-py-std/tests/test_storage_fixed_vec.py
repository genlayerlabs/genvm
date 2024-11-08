import typing

from genlayer.py.storage import Array
from genlayer.py.storage.generate import storage
from genlayer.py.types import u32
from genlayer.py.storage.generate import _known_descs

from .common import *


@storage
class StorVec:
	x: Array[u32, typing.Literal[3]]


@storage
class Regen:
	y: Array[u32, typing.Literal[3]]


def same_iter(li, ri):
	for l, r in zip(li, ri):
		assert l == r


def test_len():
	l = StorVec()
	r: list[u32] = [u32(0), u32(0), u32(0)]
	op = SameOp(l.x, r)
	same_iter(l.x, r)
	op(len)

	r[0] = u32(1)
	r[1] = u32(2)
	r[2] = u32(3)

	l.x = r  # type: ignore

	same_iter(l.x, r)

	op(lambda x: x[1])
	op(lambda x: x[-1])

	r[1] = u32(4)
	l.x[1] = u32(4)

	same_iter(l.x, r)
