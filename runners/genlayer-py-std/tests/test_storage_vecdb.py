from genlayer import VecDB
from genlayer.py.storage.generate import storage
import numpy as np
import typing

import pytest


@storage
class DB:
	x: VecDB[np.int32, typing.Literal[5], str]


def test_store_inv_shape():
	db = DB()

	with pytest.raises(Exception):
		ins_val = np.array([1], dtype=np.int32)
		db.x.insert(ins_val, '1')


def test_store_inv_type():
	db = DB()

	with pytest.raises(Exception):
		ins_val = np.array([1, 2, 3, 4, 5], dtype=np.float32)
		db.x.insert(ins_val, '1')  # type: ignore


def test_store_simple_ok():
	db = DB()

	ins_val = np.array([1, 2, 3, 4, 5], dtype=np.int32)
	db.x.insert(ins_val, '1')


# def test_store_rnn():
# db = DB()
#
# ins_val = np.array([0, 0, 0, 0, 0], dtype=np.int32)
# db.x.insert(ins_val, '0')
# ins_val = np.array([2, 0, 0, 0, 0], dtype=np.int32)
# db.x.insert(ins_val, '2')
# ins_val = np.array([-2, 0, 0, 0, 0], dtype=np.int32)
# db.x.insert(ins_val, '-2')
#
# seen = set()
# for elem in db.x.rnn(np.array([0, 0, 0, 0, 0], dtype=np.int32), np.int32(1)):
# seen.add(elem.value)
# assert seen == set(['0'])
#
# seen = set()
# for elem in db.x.rnn(np.array([0, 0, 0, 0, 0], dtype=np.int32), np.int32(3)):
# seen.add(elem.value)
# assert seen == set(['0', '2', '-2'])


def test_store_knn():
	db = DB()

	ins_val = np.array([0, 0, 0, 0, 0], dtype=np.int32)
	db.x.insert(ins_val, '0')
	ins_val = np.array([1, 0, 0, 0, 0], dtype=np.int32)
	db.x.insert(ins_val, '1')
	ins_val = np.array([2, 0, 0, 0, 0], dtype=np.int32)
	db.x.insert(ins_val, '2')

	seen = set()
	for elem in db.x.knn(np.array([0, 0, 0, 0, 0], dtype=np.int32), 1):
		seen.add(elem.value)
	assert seen == set(['0'])

	seen = set()
	for elem in db.x.knn(np.array([0, 0, 0, 0, 0], dtype=np.int32), 2):
		seen.add(elem.value)
	assert seen == set(['0', '1'])

	seen = set()
	for elem in db.x.knn(np.array([0, 0, 0, 0, 0], dtype=np.int32), 3):
		seen.add(elem.value)
	assert seen == set(['0', '1', '2'])

	seen = set()
	for elem in db.x.knn(np.array([0, 0, 0, 0, 0], dtype=np.int32), 8):
		seen.add(elem.value)
	assert seen == set(['0', '1', '2'])
