import typing
import pytest

from genlayer.storage.core import (
	Indirection,
	VLA,
	InmemManager,
	ROOT_SLOT_ID,
)
from genlayer.storage._internal.generate import _storage_build, _BuilderCtx
from genlayer.types import u32


def new_vla():
	td = _storage_build(_BuilderCtx.empty(), VLA[u32])
	man = InmemManager()
	return td.get(man.get_store_slot(ROOT_SLOT_ID), 0)


def new_indirection():
	td = _storage_build(_BuilderCtx.empty(), Indirection[u32])
	man = InmemManager()
	return td.get(man.get_store_slot(ROOT_SLOT_ID), 0)


# === VLA tests ===


def test_vla_empty():
	v = new_vla()
	assert len(v) == 0


def test_vla_append():
	v = new_vla()
	v.append(10)
	v.append(20)
	v.append(30)
	assert len(v) == 3
	assert v[0] == 10
	assert v[1] == 20
	assert v[2] == 30


def test_vla_setitem():
	v = new_vla()
	v.append(1)
	v.append(2)
	v[0] = 99
	assert v[0] == 99
	assert v[1] == 2


def test_vla_iter():
	v = new_vla()
	v.append(5)
	v.append(6)
	v.append(7)
	assert list(v) == [5, 6, 7]


def test_vla_extend():
	v = new_vla()
	v.append(1)
	v.append(2)
	v2 = new_vla()
	v2.append(10)
	v2.append(20)
	v2.append(30)
	v.extend(v2)
	assert len(v) == 5
	assert list(v) == [1, 2, 10, 20, 30]


def test_vla_truncate():
	v = new_vla()
	v.append(1)
	v.append(2)
	v.append(3)
	v.truncate(1)
	assert len(v) == 1
	assert v[0] == 1


def test_vla_truncate_to_zero():
	v = new_vla()
	v.append(1)
	v.append(2)
	v.truncate()
	assert len(v) == 0


def test_vla_truncate_out_of_range():
	v = new_vla()
	v.append(1)
	with pytest.raises(IndexError):
		v.truncate(5)


def test_vla_getitem_out_of_range():
	v = new_vla()
	v.append(1)
	with pytest.raises(IndexError):
		v[1]
	with pytest.raises(IndexError):
		v[-1]


def test_vla_setitem_out_of_range():
	v = new_vla()
	v.append(1)
	with pytest.raises(IndexError):
		v[1] = 99
	with pytest.raises(IndexError):
		v[-1] = 99


def test_vla_slot():
	v = new_vla()
	s = v.slot()
	assert s is not None


# === Indirection tests ===


def test_indirection_init_raises():
	with pytest.raises(TypeError):
		Indirection()


def test_indirection_set_get():
	ind = new_indirection()
	ind.set(42)
	assert ind.get() == 42


def test_indirection_overwrite():
	ind = new_indirection()
	ind.set(10)
	ind.set(20)
	assert ind.get() == 20


def test_indirection_slot():
	ind = new_indirection()
	s = ind.slot()
	assert s is not None
