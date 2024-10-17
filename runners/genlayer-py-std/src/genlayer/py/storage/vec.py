import typing

from .core import *
from .core import _FakeStorageMan
from .desc_base_types import _u32_desc


class WithItemStorageSlot(WithStorageSlot):
	_item_desc: TypeDesc


class Vec[T](WithItemStorageSlot):
	def __init__(self):
		raise Exception("this class can't be instantiated")

	@staticmethod
	def _view_at(item_desc: TypeDesc, slot: StorageSlot, off: int) -> 'Vec':
		slf = Vec.__new__(Vec)
		slf._item_desc = item_desc
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
		to_indirect = to.indirect(to_off)
		frm_indirect = frm.indirect(frm_off)
		if len(cop) == 1 and isinstance(cop[0], int):
			to_indirect.write(0, frm_indirect.read(0, cop[0] * le))
		else:
			cum_off = 0
			for _i in range(le):
				cum_off += actions_apply_copy(cop, to_indirect, cum_off, frm_indirect, cum_off)
		return _u32_desc.size

	def __repr__(self):
		return '_VecCopyAction'


class _VecDesc(TypeDesc):
	_item_desc: TypeDesc

	def __init__(self, it_desc: TypeDesc):
		self._item_desc = it_desc
		self._cop = _VecCopyAction(it_desc)
		TypeDesc.__init__(self, _u32_desc.size, [self._cop])

	def get(self, slot: StorageSlot, off: int) -> Vec:
		return Vec._view_at(self._item_desc, slot, off)

	def set(self, slot: StorageSlot, off: int, val: Vec | list) -> None:
		if isinstance(val, list):
			_u32_desc.set(slot, off, len(val))
			indirect_slot = slot.indirect(off)
			for i in range(len(val)):
				self._item_desc.set(indirect_slot, i * self._item_desc.size, val[i])
			return
		if val._item_desc is not self:
			raise Exception('incompatible vector type')
		self._cop.copy(val._storage_slot, val._off, slot, off)

	def __eq__(self, r):
		if not isinstance(r, _VecDesc):
			return False
		return self._item_desc == r._item_desc

	def __hash__(self):
		return hash(('_VecDesc', hash(self._item_desc)))

	def __repr__(self):
		return f'_VecDesc[{self._item_desc!r}]'
