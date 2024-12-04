import pytest

from genlayer.py.storage import DynArray
from genlayer.py.storage.generate import storage

from .common import *


@storage
class StorVec:
	x: DynArray[str]


def same_iter(li, ri):
	for l, r in zip(li, ri, strict=True):
		assert l == r


def test_len():
	l = StorVec()
	r: list[str] = []
	op = SameOp(l.x, r)
	same_iter(l.x, r)
	op(len)
	op(lambda x: x.append('123'))
	op(len)
	op(lambda x: x[0])
	op(lambda x: x[-1])
	same_iter(l.x, r)
	for i in range(5):
		op(str(i))
	same_iter(l.x, r)
	while len(r) > 0:
		op(lambda x: x.pop(), void=True)
		same_iter(l.x, r)


@pytest.mark.parametrize(
	'idx',
	[
		0,
		-1,
		4,
		-5,
	],
)
def test_setitem_int(idx: int):
	l = StorVec()
	r: list[str] = [str(x) for x in range(10)]
	l.x[:] = r
	same_iter(l.x, r)

	val = 'test'
	r[idx] = val
	l.x[idx] = val
	same_iter(l.x, r)


@pytest.mark.parametrize(
	'idx',
	[
		slice(None, None, None),
		slice(None, None, 2),
		slice(None, None, 3),
		slice(1, 3, 2),
		slice(1, 5, 2),
		slice(1, 5, 1),
		slice(1, -1, 1),
		slice(None, None, -1),
		slice(4, 8, -1),
		slice(8, 4, -1),
		slice(8, 4, -2),
		slice(8, 3, -2),
		slice(9, 3, -2),
		slice(8, 4, -3),
		slice(9, 2, -3),
	],
)
def test_setitem_slice(idx: slice):
	l = StorVec()
	r: list[str] = [str(x) for x in range(10)]
	l.x[:] = r
	same_iter(l.x, r)

	x = [str(10 + x) for x in range(5)]
	try:
		r[idx] = x
	except Exception as e:
		return
	l.x[idx] = x
	same_iter(l.x, r)


@pytest.mark.parametrize(
	'idx',
	[
		0,
		-1,
		4,
		slice(None, None, None),
		slice(None, None, 2),
		slice(None, None, 3),
		slice(1, 3, 2),
		slice(1, 5, 2),
		slice(1, 5, 1),
		slice(1, -1, 1),
		slice(None, None, -1),
		slice(4, 8, -1),
		slice(8, 4, -1),
		slice(8, 4, -2),
		slice(8, 4, -3),
	],
)
def test_getitem(idx: int | slice):
	l = StorVec()
	r: list[str] = [str(x) for x in range(10)]
	l.x[:] = r
	same_iter(l.x, r)

	same_iter(l.x[idx], r[idx])


@pytest.mark.parametrize(
	'idx',
	[
		0,
		-1,
		4,
		slice(None, None, None),
		slice(None, None, 2),
		slice(1, 3, 2),
		slice(1, 5, 2),
		slice(1, 5, 1),
		slice(1, -1, 1),
		slice(None, None, -1),
		slice(4, 8, -1),
		slice(8, 4, -1),
		slice(8, 4, -2),
		slice(8, 4, -3),
	],
)
def test_delitem(idx: int | slice):
	l = StorVec()
	r: list[str] = [str(x) for x in range(10)]
	l.x[:] = r
	same_iter(l.x, r)

	del l.x[idx]
	del r[idx]

	same_iter(l.x, r)


@pytest.mark.parametrize(
	'idx',
	[
		0,
		-1,
		4,
		-5,
	],
)
def test_insert(idx: int):
	l = StorVec()
	r: list[str] = [str(x) for x in range(10)]
	l.x[:] = r
	same_iter(l.x, r)

	val = 'test'
	r.insert(idx, val)
	l.x.insert(idx, val)
	same_iter(l.x, r)
