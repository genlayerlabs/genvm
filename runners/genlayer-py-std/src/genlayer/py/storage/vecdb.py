from __future__ import annotations

__all__ = ('VecDB', 'VecDBElement')

from . import DynArray, TreeMap
from ..types import u32

import typing
import numpy as np


def cosine_distance_fast[S, T: np.number](
	a: np.ndarray[S, np.dtype[T]], b: np.ndarray[S, np.dtype[T]]
) -> T:
	dot_product = np.dot(a, b)
	norms = np.linalg.norm(a) * np.linalg.norm(b)
	similarity = dot_product / norms
	return 1 - similarity


Id = typing.NewType('Id', int)
_Id = Id


class VecDBElement[T: np.number, S: int, V, Dist]:
	distance: Dist
	"""
	Distance from search point to this element, if any
	"""

	def __init__(self, db: VecDB[T, S, V], idx: u32, distance: Dist):
		self._idx = idx
		self._db = db
		self.distance = distance

	@property
	def key(self) -> np.ndarray[tuple[S], np.dtype[T]]:
		"""
		Key (vector) of this element
		"""
		return self._db._keys[self._idx]

	@property
	def id(self) -> Id:
		"""
		Id (unique key) of this element
		"""
		return Id(self._idx)

	@property
	def value(self) -> V:
		"""
		Value of this element
		"""
		return self._db._values[self._idx]

	@value.setter
	def value(self, v: V):
		self._db._values[self._idx] = v

	def remove(self) -> None:
		"""
		Removes current element from the db
		"""
		self._db._free_idx[self._idx] = None


class VecDB[T: np.number, S: int, V]:
	"""
	Data structure that supports storing and querying vector data

	There are two entities that can act as a key:

	#. vector (can have duplicates)
	#. id (int alias, can't have duplicates)

	.. warning::
		import :py:mod:`numpy` before ``from genlayer import *`` if you wish to use :py:class:`VecDB`!
	"""

	Id: typing.ClassVar[type] = _Id
	"""
	:py:class:`int` alias to prevent confusion
	"""

	Element = typing.ClassVar[VecDBElement]
	"""
	Shorthand to prevent global namespace pollution
	"""

	# FIXME implement production ready *NN structure
	_keys: DynArray[np.ndarray[tuple[S], np.dtype[T]]]
	_values: DynArray[V]
	_free_idx: TreeMap[u32, None]

	def __len__(self) -> int:
		return len(self._keys) - len(self._free_idx)

	def get_by_id(self, id: Id) -> VecDBElement[T, S, V, None]:
		res = self.get_by_id_or_none(id)
		if res is None:
			raise KeyError(f'no element with id {id}')
		return res

	def get_by_id_or_none(self, id: Id) -> VecDBElement[T, S, V, None] | None:
		if u32(id) in self._free_idx:
			return None
		return VecDBElement(self, u32(id), None)

	def insert(self, key: np.ndarray[tuple[S], np.dtype[T]], val: V) -> Id:
		if len(self._free_idx) > 0:
			idx = self._free_idx.popitem()[0]
			self._keys[idx] = key
			self._values[idx] = val
			return Id(idx)
		else:
			self._keys.append(key)
			self._values.append(val)
			return Id(len(self._keys) - 1)

	def _get_vecs(self, v: np.ndarray[tuple[S], np.dtype[T]]) -> list[tuple[T, int]]:
		lst: list[tuple[T, int]] = []  # dist, index
		for i in range(len(self._keys)):
			if u32(i) in self._free_idx:
				continue
			cur_key = self._keys[i]

			dist = cosine_distance_fast(cur_key, v)

			lst.append((dist, i))
		lst.sort(key=lambda x: x[0])
		return lst

	def knn(
		self, v: np.ndarray[tuple[S], np.dtype[T]], k: int
	) -> typing.Iterator[VecDBElement[T, S, V, T]]:
		for x in self._get_vecs(v):
			if k <= 0:
				return
			yield VecDBElement(self, u32(x[1]), x[0])
			k -= 1

	# def rnn(self, v: np.ndarray[tuple[S], np.dtype[T]], r: T) -> typing.Iterator[VecDBElement[T, S, V, T]]:
	# r = r * r
	# for x in self._get_vecs(v):
	# if x[0] > r:
	# return
	# yield VecDBElement(self, u32(x[1]), x[0])

	def __iter__(self):
		for i in range(len(self._keys)):
			if u32(i) in self._free_idx:
				continue
			yield VecDBElement(self, u32(i), None)
