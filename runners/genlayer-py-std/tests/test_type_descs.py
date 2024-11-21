import genlayer.py.storage.desc_base_types as base
from genlayer.py.storage.vec import _DynArrayDesc, DynArray
from genlayer.py.storage.generate import _Instantiation, _known_descs


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
	assert hash(base._IntDesc(4, False)) == hash(base._IntDesc(4, False))
	assert hash(base._IntDesc(4, False)) != hash(base._IntDesc(4, True))
	assert hash(base._IntDesc(4, False)) != hash(base._IntDesc(8, False))


def test_eq_vec():
	assert _DynArrayDesc(base._IntDesc(4, False)) == _DynArrayDesc(
		base._IntDesc(4, False)
	)
	assert _DynArrayDesc(base._IntDesc(4, False)) != _DynArrayDesc(base._IntDesc(4, True))
	assert _DynArrayDesc(base._IntDesc(4, False)) != _DynArrayDesc(
		base._IntDesc(8, False)
	)


def test_hash_vec():
	assert hash(_DynArrayDesc(base._IntDesc(4, False))) == hash(
		_DynArrayDesc(base._IntDesc(4, False))
	)
	assert hash(_DynArrayDesc(base._IntDesc(4, False))) != hash(
		_DynArrayDesc(base._IntDesc(4, True))
	)
	assert hash(_DynArrayDesc(base._IntDesc(4, False))) != hash(
		_DynArrayDesc(base._IntDesc(8, False))
	)


def test_inst():
	assert _Instantiation(DynArray, (base._IntDesc(4, False),)) == _Instantiation(
		DynArray, (base._IntDesc(4, False),)
	)
	assert _Instantiation(DynArray, (base._IntDesc(4, False),)) != _Instantiation(
		DynArray, (base._IntDesc(4, True),)
	)
	assert _Instantiation(DynArray, (base._IntDesc(4, False),)) != _Instantiation(
		DynArray, (base._IntDesc(8, False),)
	)
