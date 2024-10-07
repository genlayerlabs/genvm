import typing

from .core import *
from .desc_base_types import _u32_desc


class WithItemStorageSlot(WithStorageSlot):
	_item_desc: TypeDesc


class Vec[T](WithItemStorageSlot):
	def __init__(self):
		raise Exception("this class can't be instantiated")

	@staticmethod
	def view_at(slot: StorageSlot, off: int) -> 'Vec':
		slf = Vec.__new__(Vec)
		slf._storage_slot = slot
		slf._off = off
		return slf

	def __len__(self) -> int:
		return _u32_desc.get(self._storage_slot, self._off)

	def _map_index(self, idx: int) -> int:
		le = len(self)
		if idx < 0:
			idx += le
		if idx < 0 or idx >= le:
			raise IndexError(f'index out of range {idx} not in 0..<{le}')
		return idx

	def __getitem__(self, idx: int) -> T:
		idx = self._map_index(idx)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.get(items_at, idx * self._item_desc.size)

	def __setitem__(self, idx: int, val: T) -> None:
		idx = self._map_index(idx)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.set(items_at, idx * self._item_desc.size, val)

	def __iter__(self) -> typing.Any:
		for i in range(len(self)):
			yield self[i]

	def append(self, item: T) -> None:
		le = len(self)
		_u32_desc.set(self._storage_slot, self._off, le + 1)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.set(items_at, le * self._item_desc.size, item)

	def append_new_get(self) -> T:
		le = len(self)
		_u32_desc.set(self._storage_slot, self._off, le + 1)
		items_at = self._storage_slot.indirect(self._off)
		return self._item_desc.get(items_at, le * self._item_desc.size)

	def pop(self) -> None:
		le = len(self)
		if le == 0:
			raise Exception("can't pop from empty array")
		_u32_desc.set(self._storage_slot, self._off, le - 1)


class _VecCopyAction(ComplexCopyAction):
	def __init__(self, item_desc: TypeDesc):
		self._item_desc = item_desc

	def copy(self, frm: StorageSlot, frm_off: int, to: StorageSlot, to_off: int) -> int:
		le = _u32_desc.get(frm, frm_off)
		_u32_desc.set(to, to_off, le)
		cop = self._item_desc.copy_actions
		to_indir = to.indirect(to_off)
		frm_indir = frm.indirect(frm_off)
		if len(cop) == 1 and isinstance(cop[0], int):
			to_indir.write(0, frm_indir.read(0, self._item_desc.size * le))
		else:
			cum_off = 0
			for i in range(le):
				cum_off += actions_apply_copy(cop, to, to_off + cum_off, frm, frm_off + cum_off)
		return _u32_desc.size


class _VecDescription(TypeDesc):
	_item_desc: TypeDesc

	def __init__(self, it_desc: TypeDesc):
		self._item_desc = it_desc
		self._cop = _VecCopyAction(it_desc)
		TypeDesc.__init__(self, _u32_desc.size, [self._cop])

	def get(self, slot: StorageSlot, off: int) -> Vec:
		res = Vec.view_at(slot, off)
		res._item_desc = self._item_desc
		return res

	def set(self, slot: StorageSlot, off: int, val: Vec) -> None:
		if val._item_desc is not self:
			raise Exception('incompatible vector type')
		self._cop.copy(val._storage_slot, val._off, slot, off)
