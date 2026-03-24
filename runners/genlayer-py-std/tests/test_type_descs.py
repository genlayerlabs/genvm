import genlayer.storage._internal.desc_base_types as base
from genlayer.storage.dyn_array import _DynArrayDesc, DynArray
from genlayer.storage._internal.generate import _known_descs


def test_all_different():
	l = iter(_known_descs)
	r = iter(_known_descs)
	next(r)
	for a, b in zip(l, r):
		assert a != b
		assert a == a
		assert b == b


def test_all_different_hashes():
	l = iter(_known_descs)
	r = iter(_known_descs)
	next(r)
	for a, b in zip(l, r):
		assert hash(a) != hash(b)


def test_hash_int():
	assert hash(base.IntDesc(4, False)) == hash(base.IntDesc(4, False))
	assert hash(base.IntDesc(4, False)) != hash(base.IntDesc(4, True))
	assert hash(base.IntDesc(4, False)) != hash(base.IntDesc(8, False))


def test_eq_vec():
	assert _DynArrayDesc(base.IntDesc(4, False)) == _DynArrayDesc(base.IntDesc(4, False))
	assert _DynArrayDesc(base.IntDesc(4, False)) != _DynArrayDesc(base.IntDesc(4, True))
	assert _DynArrayDesc(base.IntDesc(4, False)) != _DynArrayDesc(base.IntDesc(8, False))


def test_hash_vec():
	assert hash(_DynArrayDesc(base.IntDesc(4, False))) == hash(
		_DynArrayDesc(base.IntDesc(4, False))
	)
	assert hash(_DynArrayDesc(base.IntDesc(4, False))) != hash(
		_DynArrayDesc(base.IntDesc(4, True))
	)
	assert hash(_DynArrayDesc(base.IntDesc(4, False))) != hash(
		_DynArrayDesc(base.IntDesc(8, False))
	)
