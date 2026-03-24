import pytest

from genlayer.types import Lazy


def test_get():
	lazy = Lazy(lambda: 42)
	assert lazy.get() == 42


def test_cached():
	calls = 0

	def fn():
		nonlocal calls
		calls += 1
		return 'result'

	lazy = Lazy(fn)
	assert lazy.get() == 'result'
	assert lazy.get() == 'result'
	assert calls == 1


def test_exception():
	lazy = Lazy(lambda: (_ for _ in ()).throw(ValueError('boom')))
	with pytest.raises(ValueError, match='boom'):
		lazy.get()


def test_exception_cached():
	err = ValueError('fail')
	lazy = Lazy(lambda: (_ for _ in ()).throw(err))
	with pytest.raises(ValueError):
		lazy.get()
	with pytest.raises(ValueError):
		lazy.get()
