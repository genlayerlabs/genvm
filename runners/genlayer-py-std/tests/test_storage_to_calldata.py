from genlayer.py.storage import DynArray, Array, TreeMap
from genlayer.py.storage.generate import storage
from dataclasses import dataclass
import typing

import genlayer.py.calldata as calldata

from .common import *


@storage
class Store:
	da: DynArray[str]
	sa: Array[str, typing.Literal[3]]
	mp: TreeMap[str, str]


def test_dyn_array():
	x = Store()

	assert calldata.decode(calldata.encode(x.da)) == []
	x.da.append('1')
	x.da.append('2')
	assert calldata.decode(calldata.encode(x.da)) == ['1', '2']


def test_array():
	x = Store()

	assert calldata.decode(calldata.encode(x.sa)) == ['', '', '']
	for i in range(3):
		x.sa[i] = str((i + 1) ** 2)
	assert calldata.decode(calldata.encode(x.sa)) == ['1', '4', '9']


def test_tree_map():
	x = Store()

	assert calldata.decode(calldata.encode(x.mp)) == {}
	for i in range(3):
		x.mp[str(i + 1)] = str((i + 1) ** 2)
	assert calldata.decode(calldata.encode(x.mp)) == {'1': '1', '2': '4', '3': '9'}


@dataclass
class DC:
	x: int
	y: str


def test_dataclass():
	dc = DC(10, 'q')
	assert calldata.decode(calldata.encode(dc)) == {
		'x': 10,
		'y': 'q',
	}


class Strange:
	def __to_calldata__(self):
		return 11


def test_class_override():
	assert calldata.decode(calldata.encode(Strange())) == 11


def test_default():
	assert calldata.decode(calldata.encode(Strange(), default=lambda _x: 12)) == 12
