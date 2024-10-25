from .vec import Array, DynArray
from .core import WithStorageSlot

import abc
import typing


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


def euclid_distance_squared[T: Numeric](
	l: typing.Iterator[T], r: typing.Iterator[T]
) -> T:
	dist: T | None = None
	for x, y in zip(l, r):
		tmp = (x - y) ** 2
		if dist is None:
			dist = tmp
		else:
			dist += tmp
	assert dist is not None
	return dist


class VecDB[T: Numeric, S: int, V](WithStorageSlot):
	# FIXME
	_keys: DynArray[Array[T, S]]
	_values: DynArray[V]

	def __len__(self) -> int:
		return len(self._keys)

	def insert(self, key: Array[T, S], val: V):
		self._keys.append(key)
		self._values.append(val)

	def _get_vecs(self, v: Array[T, S]) -> list[tuple[T, int]]:
		lst: list[tuple[T, int]] = []  # dist, index
		for i in range(len(self)):
			dist = euclid_distance_squared(iter(self._keys[i]), iter(v))
			lst.append((dist, i))
		return sorted(lst, key=lambda x: x[0])

	def knn(self, v: Array[T, S], k: int, /) -> typing.Iterator[tuple[Array[T, S], V]]:
		for x in self._get_vecs(v):
			if k <= 0:
				return
			yield (self._keys[x[1]], self._values[x[1]])
			k -= 1

	def rnn(self, v: Array[T, S], r: T, /) -> typing.Iterator[tuple[Array[T, S], V]]:
		for x in self._get_vecs(v):
			if x[0] > r:
				return
			yield (self._keys[x[1]], self._values[x[1]])
