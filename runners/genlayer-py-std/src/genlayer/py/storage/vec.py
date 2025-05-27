__all__ = ('DynArray', 'Array')

import typing

from ._internal.core import *
from ._internal.core import _WithStorageSlotAndTD

from ._internal.desc_base_types import _u32_desc

from ..types import SizedArray


class DynArray[T](_WithStorageSlotAndTD, collections.abc.MutableSequence[T]):
	"""
	Represents exponentially growing array (:py:class:`list` in python terms) that can be persisted on the blockchain
	"""

	_item_desc: TypeDesc

	__slots__ = ('_item_desc', '_off', '_storage_slot')

	def __init__(self):
		"""
		This class can't be created with ``DynArray()``

		:raises TypeError: always
		"""
		raise TypeError("this class can't be instantiated by user")

	def __len__(self) -> int:
		return _u32_desc.get(self._storage_slot, self._off)

	def _map_index(self, idx: int) -> int:
		le = len(self)
		if idx < 0:
			idx += le
		if idx < 0 or idx >= le:
			raise IndexError(f'index out of range {idx} not in 0..<{le}')
		return idx

	@typing.overload
	def __getitem__(self, idx: int) -> T: ...
	@typing.overload
	def __getitem__(self, idx: slice) -> list[T]: ...

	def __getitem__(self, idx: int | slice) -> T | list[T]:
		if isinstance(idx, int):
			idx = self._map_index(idx)
			items_at = self._storage_slot.indirect(self._off)
			return self._item_desc.get(items_at, idx * self._item_desc.size)
		else:
			start, stop, step = idx.indices(len(self))
			ret = []
			step_sign = 1 if step >= 0 else -1
			while start * step_sign < stop * step_sign:
				ret.append(self[start])
				start += step
			return ret

	@typing.overload
	def __setitem__(self, idx: typing.SupportsIndex, val: T) -> None: ...
	@typing.overload
	def __setitem__(self, idx: slice, val: collections.abc.Sequence[T]) -> None: ...

	def __setitem__(
		self, idx: typing.SupportsIndex | slice, val: T | collections.abc.Sequence[T]
	) -> None:
		if not isinstance(idx, slice):
			idx = self._map_index(idx.__index__())
			items_at = self._storage_slot.indirect(self._off)
			self._item_desc.set(items_at, idx * self._item_desc.size, val)
			return
		else:
			start, stop, step = self._slice_to_idx(idx)
			new_val = typing.cast(collections.abc.Sequence[T], val)
			left_in_new = len(new_val)
			if isinstance(idx.step, int) and idx.step < 0:
				new_val = reversed(new_val)
			left_in_range = (stop - start) // step
			new_it = iter(new_val)

			# just reassign existing values
			common_values_cnt = min(left_in_new, left_in_range)
			for i in range(common_values_cnt):
				self[start + i * step] = next(new_it)

			start += common_values_cnt
			left_in_range -= common_values_cnt
			left_in_new -= common_values_cnt

			# if we have other values we must remove them
			if left_in_range > 0:
				del self[start:stop:step]

			# if we have some unassigned we must insert it here
			elif left_in_new > 0:
				# move current to the right
				items_at = self._storage_slot.indirect(self._off)
				for i in range(len(self) - 1, start - 1, -1):
					self._item_desc.set(
						items_at, (i + left_in_new) * self._item_desc.size, self[i]
					)
				for i in range(left_in_new):
					self._item_desc.set(
						items_at, (start + i) * self._item_desc.size, next(new_it)
					)
				_u32_desc.set(self._storage_slot, self._off, len(self) + left_in_new)

	def _slice_to_idx(self, s: slice) -> tuple[int, int, int]:
		start, stop, step = s.indices(len(self))
		if step < 0:
			step *= -1
			start, stop = stop, start
			# stop += (step - (stop - start) % step) % step
			start = stop - (stop - start - 1) // step * step
			stop += 1
		return start, stop, step

	@typing.overload
	def __delitem__(self, idx: int) -> None: ...
	@typing.overload
	def __delitem__(self, idx: slice) -> None: ...

	def __delitem__(self, idx: int | slice) -> None:
		if isinstance(idx, int):
			start = self._map_index(idx)
			stop = start + 1
			step = 1
		else:
			start, stop, step = self._slice_to_idx(idx)
		if stop <= start:
			return
		next_deletion = start
		insert_idx = start
		for i in range(start, len(self)):
			if i == next_deletion:
				next_deletion = i + step
				if next_deletion >= stop:
					next_deletion = -1
				continue
			self[insert_idx] = self[i]
			insert_idx += 1
		_u32_desc.set(self._storage_slot, self._off, insert_idx)

	def insert(self, index: int, value: T) -> None:
		index = self._map_index(index)
		old_len = len(self)
		_u32_desc.set(self._storage_slot, self._off, old_len + 1)
		for i in range(old_len, index, -1):
			self[i] = self[i - 1]
		self[index] = value

	def __iter__(self) -> typing.Any:
		for i in range(len(self)):
			yield self[i]

	def append(self, value: T) -> None:
		le = len(self)
		_u32_desc.set(self._storage_slot, self._off, le + 1)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.set(items_at, le * self._item_desc.size, value)

	def append_new_get(self) -> T:
		le = len(self)
		_u32_desc.set(self._storage_slot, self._off, le + 1)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.get(items_at, le * self._item_desc.size)

	def pop(self) -> None:  # type: ignore
		le = len(self)
		if le == 0:
			raise Exception("can't pop from empty array")
		_u32_desc.set(self._storage_slot, self._off, le - 1)

	def __repr__(self) -> str:
		ret: list[str] = []
		ret.append('[')
		comma = False
		for x in self:
			if comma:
				ret.append(',')
			comma = True
			ret.append(repr(x))
		ret.append(']')
		return ''.join(ret)


class _DynArrayDesc(SpecialTypeDesc, ComplexCopyAction):
	_item_desc: TypeDesc

	__slots__ = ('_item_desc',)

	def __init__(self, item_desc: TypeDesc):
		SpecialTypeDesc.__init__(self, item_desc, lambda: DynArray.__new__(DynArray))
		TypeDesc.__init__(self, _u32_desc.size, [self])

	def copy(self, frm: Slot, frm_off: int, to: Slot, to_off: int) -> int:
		le = _u32_desc.get(frm, frm_off)
		_u32_desc.set(to, to_off, le)

		cop = self._item_desc.copy_actions
		to_indirect = to.indirect(to_off)
		frm_indirect = frm.indirect(frm_off)
		if len(cop) == 1 and isinstance(cop[0], int):
			to_indirect.write(0, frm_indirect.read(0, cop[0] * le))
		else:
			cum_off = 0
			for _i in range(le):
				cum_off += actions_apply_copy(cop, to_indirect, cum_off, frm_indirect, cum_off)
		return _u32_desc.size

	def set(self, slot: Slot, off: int, val: DynArray | collections.abc.Sequence) -> None:
		if isinstance(val, DynArray):
			if val._item_desc is not self:
				raise TypeError('incompatible vector type')
			self.copy(val._storage_slot, val._off, slot, off)
			return

		_u32_desc.set(slot, off, len(val))
		indirect_slot = slot.indirect(off)
		for i in range(len(val)):
			self._item_desc.set(indirect_slot, i * self._item_desc.size, val[i])
		return

	def __eq__(self, r):
		if not isinstance(r, _DynArrayDesc):
			return False
		return self._item_desc == r._item_desc

	def __hash__(self):
		return hash(('_VecDesc', hash(self._item_desc)))

	def __repr__(self):
		return f'_VecDesc[{self._item_desc!r}]'


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
		idx = self._map_index(idx)
		self._item_desc.set(self._storage_slot, self._off + idx * self._item_desc.size, val)

	def __iter__(self):
		for i in range(len(self)):
			yield self[i]


class _ArrayDesc(SpecialTypeDesc):
	_item_desc: TypeDesc
	_len: int

	__slots__ = ('_item_desc', '_len')

	def __init__(self, item_desc: TypeDesc, le: int):
		SpecialTypeDesc.__init__(self, item_desc, lambda: Array.__new__(Array))

		cop: list[CopyAction] = []
		for _i in range(le):
			actions_append(cop, item_desc.copy_actions)

		TypeDesc.__init__(self, le * item_desc.size, cop)

	def set(self, slot: Slot, off: int, val: Array | list) -> None:
		assert len(val) == self._len
		if isinstance(val, list):
			for i in range(self._len):
				self._item_desc.set(slot, off + i * self._item_desc.size, val[i])
		else:
			actions_apply_copy(self.copy_actions, slot, off, val._storage_slot, val._off)

	def __eq__(self, r):
		if not isinstance(r, Array):
			return False
		return self._item_desc == r._item_desc and self._len == r._len

	def __hash__(self):
		return hash(('_FixedVecDesc', hash(self._item_desc), self._len))

	def __repr__(self):
		return f'_FixedVecDesc[{self._item_desc!r}, {self._len}]'
