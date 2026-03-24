__all__ = ('Array',)

import typing
import collections.abc

from .core import (
	_WithStorageSlotAndTD,
	TypeDesc,
	Slot,
	CopyAction,
	actions_apply_copy,
	SpecialTypeDesc,
	actions_append,
)

from ..types import SizedArray


class Array[T, S: int](
	_WithStorageSlotAndTD, collections.abc.Sequence, SizedArray[T, S]
):
	"""
	Constantly sized array that can be persisted on the blockchain
	"""

	_item_desc: TypeDesc
	_len: int

	__slots__ = ('_item_desc', '_len', '_off', '_storage_slot')

	def __init__(self):
		"""
		This class can't be created with ``Array()``

		:raises TypeError: always
		"""
		raise TypeError('this class can not be instantiated by user')

	def __len__(self) -> int:
		return self._len

	@staticmethod
	def _view_at(item_desc: TypeDesc, le: int, slot: Slot, off: int) -> 'Array':
		slf = Array.__new__(Array)
		slf._item_desc = item_desc
		slf._len = le
		slf._storage_slot = slot
		slf._off = off
		return slf

	def _map_index(self, idx: int) -> int:
		le = len(self)
		if idx < 0:
			idx += le
		if idx < 0 or idx >= le:
			raise IndexError(f'index out of range {idx} not in 0..<{le}')
		return idx

	@typing.overload
	def __getitem__(self, idx: typing.SupportsIndex) -> T: ...
	@typing.overload
	def __getitem__(self, idx: slice) -> 'Array': ...

	def __getitem__(self, idx: typing.SupportsIndex | slice) -> T | 'Array':
		"""
		Get element by index or a view over a sub-range by slice.

		:param idx: integer index or slice (step must be 1)
		:returns: single element for int index, ``Array`` view for slice
		:raises IndexError: when integer index is out of range
		:raises KeyError: when slice step is not 1
		"""
		if not isinstance(idx, slice):
			idx = self._map_index(idx.__index__())
			return self._item_desc.get(
				self._storage_slot, self._off + idx * self._item_desc.size
			)
		else:
			start, stop, step = idx.indices(len(self))
			if step != 1:
				raise KeyError('negative step is not supported')
			le = max(stop - start, 0)
			return Array._view_at(
				self._item_desc,
				le,
				self._storage_slot,
				self._off + start * self._item_desc.size,
			)

	def __setitem__(self, idx: int, val: T) -> None:
		"""
		Set element at the given index.

		:param idx: integer index (supports negative indexing)
		:param val: value to set
		:raises IndexError: when index is out of range
		"""
		idx = self._map_index(idx)
		self._item_desc.set(self._storage_slot, self._off + idx * self._item_desc.size, val)

	def __iter__(self):
		for i in range(len(self)):
			yield self[i]


class _ArrayDesc(SpecialTypeDesc):
	_len: int

	__slots__ = ('item_desc', 'view_ctor', '_len')

	def __init__(self, item_desc: TypeDesc, le: int):
		self._len = le

		def new_array():
			ret = Array.__new__(Array)
			ret._len = le
			return ret

		SpecialTypeDesc.__init__(self, item_desc, new_array)

		cop: list[CopyAction] = []
		for _i in range(le):
			actions_append(cop, item_desc.copy_actions)

		TypeDesc.__init__(self, le * item_desc.size, cop)

	def set(self, slot: Slot, off: int, val: Array | list) -> None:
		assert len(val) == self._len
		if isinstance(val, list):
			for i in range(self._len):
				self.item_desc.set(slot, off + i * self.item_desc.size, val[i])
		else:
			actions_apply_copy(self.copy_actions, slot, off, val._storage_slot, val._off)
