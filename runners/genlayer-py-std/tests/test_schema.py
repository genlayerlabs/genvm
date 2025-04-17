from genlayer.py.get_schema import get_schema
import genlayer.py.get_schema as _get_schema

import typing


def public(f):
	setattr(f, _get_schema.PUBLIC_ATTR, True)
	return f


def public_view(f):
	setattr(f, _get_schema.PUBLIC_ATTR, True)
	setattr(f, _get_schema.READONLY_ATTR, True)
	return f


class A:
	def __init__(self, x: int, *, y: str): ...

	@public
	def foo(self, x: dict, *, y: list): ...

	@public_view
	def foo_bar(self, x: dict[str, int], *, y: list[list[int]]): ...

	@public_view
	def an(self, x: typing.Any): ...


def test_class():
	print(get_schema(A))
	assert get_schema(A) == {
		'ctor': {'params': [['x', 'int']], 'kwparams': {'y': 'string'}},
		'methods': {
			'an': {
				'params': [['x', 'any']],
				'kwparams': {},
				'readonly': True,
				'ret': 'any',
			},
			'foo': {
				'params': [['x', 'dict']],
				'kwparams': {'y': 'array'},
				'readonly': False,
				'payable': False,
				'ret': 'any',
			},
			'foo_bar': {
				'params': [['x', {'$dict': 'int'}]],
				'kwparams': {'y': [{'$rep': [{'$rep': 'int'}]}]},
				'readonly': True,
				'ret': 'any',
			},
		},
	}


from dataclasses import dataclass


@dataclass
class B_data:
	x: int
	y: int
	z: typing.Literal['str']


class B:
	def __init__(self):
		pass

	@public_view
	def tst(self) -> B_data: ...


def test_dataclass():
	print(get_schema(B))
	assert get_schema(B) == {
		'ctor': {'params': [], 'kwparams': {}},
		'methods': {
			'tst': {
				'params': [],
				'kwparams': {},
				'readonly': True,
				'ret': {'x': 'int', 'y': 'int', 'z': 'any'},
			}
		},
	}
