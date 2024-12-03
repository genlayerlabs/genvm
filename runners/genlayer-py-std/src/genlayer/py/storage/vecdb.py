from __future__ import annotations

from . import Array, DynArray, TreeMap
from .core import WithStorageSlot
from ..types import u32

import abc
import typing
import numpy as np


class Numeric(typing.Protocol):
	@abc.abstractmethod
	def __eq__(self, other: typing.Any, /) -> bool: ...

	@abc.abstractmethod
	def __lt__(self, other: typing.Any, /) -> bool: ...

	@abc.abstractmethod
	def __gt__(self, other: typing.Any, /) -> bool: ...

	@abc.abstractmethod
	def __sub__(self, other: typing.Any, /) -> typing.Any: ...

	@abc.abstractmethod
	def __mul__(self, other: typing.Any, /) -> typing.Any: ...


def cosine_distance_fast[S, T: np.number](
	a: np.ndarray[S, np.dtype[T]], b: np.ndarray[S, np.dtype[T]]
) -> T:
	dot_product = np.dot(a, b)
	norms = np.linalg.norm(a) * np.linalg.norm(b)
	similarity = dot_product / norms
	return 1 - similarity


class VecDB[T: np.number, S: int, V]:
	"""
	Data structure that supports storing and querying vector data
	"""

	# FIXME implement production ready *NN structure
	_keys: DynArray[np.ndarray[tuple[S], np.dtype[T]]]
	_values: DynArray[V]
	_free_idx: TreeMap[u32, None]

	def __len__(self) -> int:
		return len(self._keys) - len(self._free_idx)

	def insert(self, key: np.ndarray[tuple[S], np.dtype[T]], val: V):
		if len(self._free_idx) > 0:
			idx = next(iter(self._free_idx))
			del self._free_idx[idx]
			self._keys[idx] = key
			self._values[idx] = val
		else:
			self._keys.append(key)
			self._values.append(val)

	def _get_vecs(self, v: np.ndarray[tuple[S], np.dtype[T]]) -> list[tuple[T, int]]:
		lst: list[tuple[T, int]] = []  # dist, index
		for i in range(len(self._keys)):
			if i in self._free_idx:
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
			if i in self._free_idx:
				continue
			yield VecDBElement(self, u32(i), None)


class VecDBElement[T: np.number, S: int, V, Dist]:
	def __init__(self, db: VecDB[T, S, V], idx: u32, distance: Dist):
		self._idx = idx
		self._db = db
		self.distance = distance

	@property
	def key(self) -> np.ndarray[tuple[S], np.dtype[T]]:
		return self._db._keys[self._idx]

	@property
	def value(self) -> V:
		return self._db._values[self._idx]

	@value.setter
	def value(self, v: V):
		self._db._values[self._idx] = v

	def remove(self) -> None:
		self._db._free_idx[self._idx] = None
