import pytest

from genlayer.py._internal.event import Event


def test_invalid_sig():
	with pytest.raises(TypeError):

		class KwPosEv(Event):
			def __init__(self, a): ...

	with pytest.raises(TypeError):

		class VarArgEv(Event):
			def __init__(self, *a): ...


def test_sorts():
	class Sorts(Event):
		def __init__(self, b, a, /): ...

	assert Sorts.indexed == ('a', 'b')
	assert Sorts.signature == 'Sorts(a,b)'

	Sorts(1, 2)


def test_with_blob():
	class Sorts(Event):
		def __init__(self, b, a, /, **blob): ...

	assert Sorts.indexed == ('a', 'b')
	assert Sorts.signature == 'Sorts(a,b)'

	Sorts(1, 2, gg=5)


def test_name_override():
	class Name1(Event):
		name = 'Name2'

		def __init__(self, b, a, /, **blob): ...

	assert Name1.name == 'Name2'
	assert Name1.signature == 'Name2(a,b)'
