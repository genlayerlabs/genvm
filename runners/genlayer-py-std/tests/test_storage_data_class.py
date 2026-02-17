from genlayer.storage import allow
from genlayer.storage._internal.generate import generate_storage
from dataclasses import dataclass


@allow
@dataclass
class A:
	x: str

	def __init__(self, x: str):
		self.x = x


@generate_storage
@dataclass
class B:
	x: A
	y: A


def test_assignments_depth_1():
	b = B(A('x'), A('y'))

	assert b.y.x == 'y'
	assert b.x.x == 'x'
