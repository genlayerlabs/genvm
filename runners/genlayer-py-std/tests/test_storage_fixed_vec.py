import typing
import pytest

from genlayer.storage import Array
from genlayer.storage._internal.generate import _storage_build, _BuilderCtx
from genlayer.storage.core import InmemManager, ROOT_SLOT_ID
from genlayer.types import u32

from .common import *

ArrType = Array[u32, typing.Literal[3]]


def new_arr():
	td = _storage_build(_BuilderCtx.empty(), ArrType)
	man = InmemManager()
	return td.get(man.get_store_slot(ROOT_SLOT_ID), 0)


def same_iter(li, ri):
	for l, r in zip(li, ri):
		assert l == r


def test_len():
	lx = new_arr()
	r: list[u32] = [0, 0, 0]
	op = SameOp(lx, r)
	same_iter(lx, r)
	op(len)

	r[0] = 1
	r[1] = 2
	r[2] = 3

	lx[0] = 1
	lx[1] = 2
	lx[2] = 3

	same_iter(lx, r)

	op(lambda x: x[1])
	op(lambda x: x[-1])

	r[1] = 4
	lx[1] = 4

	same_iter(lx, r)


def test_init_raises():
	with pytest.raises(TypeError):
		Array()


def test_getitem_int():
	lx = new_arr()
	lx[0] = 10
	lx[1] = 20
	lx[2] = 30
	assert lx[0] == 10
	assert lx[1] == 20
	assert lx[2] == 30


def test_getitem_negative():
	lx = new_arr()
	lx[0] = 10
	lx[1] = 20
	lx[2] = 30
	assert lx[-1] == 30
	assert lx[-2] == 20
	assert lx[-3] == 10


def test_getitem_slice():
	lx = new_arr()
	lx[0] = 10
	lx[1] = 20
	lx[2] = 30

	sub = lx[0:2]
	assert len(sub) == 2
	assert sub[0] == 10
	assert sub[1] == 20


def test_getitem_slice_empty():
	lx = new_arr()
	sub = lx[1:1]
	assert len(sub) == 0


def test_getitem_slice_full():
	lx = new_arr()
	lx[0] = 1
	lx[1] = 2
	lx[2] = 3

	sub = lx[:]
	assert len(sub) == 3
	same_iter(sub, [1, 2, 3])


def test_getitem_slice_negative_step_raises():
	lx = new_arr()
	with pytest.raises(KeyError):
		lx[::-1]


def test_setitem():
	lx = new_arr()
	lx[0] = 99
	assert lx[0] == 99
	lx[-1] = 88
	assert lx[2] == 88


def test_iter():
	lx = new_arr()
	lx[0] = 5
	lx[1] = 6
	lx[2] = 7
	result = list(lx)
	assert result == [5, 6, 7]


@pytest.mark.parametrize('idx', [3, -4, 100])
def test_getitem_out_of_range(idx: int):
	lx = new_arr()
	with pytest.raises(IndexError):
		lx[idx]


@pytest.mark.parametrize('idx', [3, -4, 100])
def test_setitem_out_of_range(idx: int):
	lx = new_arr()
	with pytest.raises(IndexError):
		lx[idx] = 0


def test_slice_is_view():
	lx = new_arr()
	lx[0] = 1
	lx[1] = 2
	lx[2] = 3

	sub = lx[1:3]
	sub[0] = 99
	assert lx[1] == 99
