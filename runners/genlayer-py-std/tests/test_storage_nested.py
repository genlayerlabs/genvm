from genlayer.py.storage import DynArray, allow_storage
from genlayer.py.storage._internal.generate import generate_storage


@allow_storage
class A:
	x: str

	def __init__(self, x: str):
		self.x = x


@generate_storage
class B:
	x: A
	y: A


def test_assignments_depth_1():
	b = B()
	b.x = A('x')
	b.y = A('y')

	assert b.y.x == 'y'
	assert b.x.x == 'x'


@allow_storage
class C:
	v: DynArray[str]

	def __init__(self, x: list[str]):
		self.v = x  # type: ignore


@generate_storage
class D:
	x: C
	y: C


def test_assignments_depth_2():
	d = D()

	c1 = C(['1', '2'])
	c2 = C(['3', '4'])

	d.x = c1
	d.y = c2

	assert list(d.x.v) == ['1', '2']
	assert list(d.y.v) == ['3', '4']


def test_assignments_value_type():
	d = D()

	c = C(['1', '2'])

	d.x = c
	c.v.append('y')
	d.x.v.append('x')
	d.y = c

	assert list(d.x.v) == ['1', '2', 'x']
	assert list(d.y.v) == ['1', '2', 'y']
