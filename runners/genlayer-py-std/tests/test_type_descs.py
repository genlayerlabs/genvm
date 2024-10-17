import genlayer.py.storage.desc_base_types as base
from genlayer.py.storage.vec import _VecDesc, Vec
from genlayer.py.storage.generate import _Instantiation


def test_eq_int():
	assert base._IntDesc(4, False) == base._IntDesc(4, False)
	assert base._IntDesc(4, False) != base._IntDesc(4, True)
	assert base._IntDesc(4, False) != base._IntDesc(8, False)


def test_hash_int():
	assert hash(base._IntDesc(4, False)) == hash(base._IntDesc(4, False))
	assert hash(base._IntDesc(4, False)) != hash(base._IntDesc(4, True))
	assert hash(base._IntDesc(4, False)) != hash(base._IntDesc(8, False))


def test_eq_vec():
	assert _VecDesc(base._IntDesc(4, False)) == _VecDesc(base._IntDesc(4, False))
	assert _VecDesc(base._IntDesc(4, False)) != _VecDesc(base._IntDesc(4, True))
	assert _VecDesc(base._IntDesc(4, False)) != _VecDesc(base._IntDesc(8, False))


def test_hash_vec():
	assert hash(_VecDesc(base._IntDesc(4, False))) == hash(
		_VecDesc(base._IntDesc(4, False))
	)
	assert hash(_VecDesc(base._IntDesc(4, False))) != hash(
		_VecDesc(base._IntDesc(4, True))
	)
	assert hash(_VecDesc(base._IntDesc(4, False))) != hash(
		_VecDesc(base._IntDesc(8, False))
	)


def test_inst():
	assert _Instantiation(Vec, (base._IntDesc(4, False),)) == _Instantiation(
		Vec, (base._IntDesc(4, False),)
	)
	assert _Instantiation(Vec, (base._IntDesc(4, False),)) != _Instantiation(
		Vec, (base._IntDesc(4, True),)
	)
	assert _Instantiation(Vec, (base._IntDesc(4, False),)) != _Instantiation(
		Vec, (base._IntDesc(8, False),)
	)
