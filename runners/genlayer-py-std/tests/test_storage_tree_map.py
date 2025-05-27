import pytest

from genlayer import *
from genlayer.py.storage._internal.generate import generate_storage


def same_iter(li, ri):
	for l, r in zip(li, ri, strict=True):
		assert l == r


@generate_storage
class UserStorage:
	m: TreeMap[str, str]


def test_construct():
	r = {str(i): str(i) + str(i) for i in range(10)}
	l = UserStorage()
	l.m.update(r)

	same_iter(l.m.items(), r.items())


@pytest.mark.parametrize('key', ['1', '-1', '2', '10'])
def test_contains(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	l = UserStorage()
	l.m.update(r)

	assert (key in l.m) == (key in r)


@pytest.mark.parametrize(
	'key,dflt', [(key, dflt) for key in ['1', '-1', '2', '10'] for dflt in [None, 'dflt']]
)
def test_get_dflt(key: str, dflt):
	r = {str(i): str(i) + str(i) for i in range(10)}
	l = UserStorage()
	l.m.update(r)

	assert l.m.get(key, dflt) == r.get(key, dflt)


@pytest.mark.parametrize('key', ['1', '-1', '2', '10'])
def test_get(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	l = UserStorage()
	l.m.update(r)

	assert l.m.get(key) == r.get(key)


@pytest.mark.parametrize('key', ['1', '-1', '2', '10', '1000'])
def test_set(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	l = UserStorage()
	l.m.update(r)

	l.m[key] = 'test'
	r[key] = 'test'

	same_iter(sorted(l.m.items()), sorted(r.items()))


@pytest.mark.parametrize('key', ['1', '2', '9'])
def test_del(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	l = UserStorage()
	l.m.update(r)

	del l.m[key]
	del r[key]

	same_iter(sorted(l.m.items()), sorted(r.items()))
