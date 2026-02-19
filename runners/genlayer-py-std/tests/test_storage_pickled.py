import pytest

from genlayer.storage._internal.generate import generate_storage
import genlayer as gl


@generate_storage
class Store:
	x: gl.storage.Pickled[dict]
	y: gl.storage.Pickled[list]
	z: gl.storage.Pickled[set]
	w: gl.storage.Pickled[tuple]


def test_pickled():
	st = Store()
	st.x.store({'a': 1, 'b': 2})
	st.y.store([1, 2, 3])
	st.z.store({1, 2, 3})
	st.w.store((1, 2, 3))
	assert st.x.load() == {'a': 1, 'b': 2}
	assert st.y.load() == [1, 2, 3]
	assert st.z.load() == {1, 2, 3}
	assert st.w.load() == (1, 2, 3)
