import typing

from .core import *
from .core import _WithStorageSlot


class _RecordDesc[T: WithRecordStorageSlot](TypeDesc):
	props: dict[str, tuple[TypeDesc, int]]

	__slots__ = ('props', 'view_ctor', 'hsh')

	def __init__(
		self,
		view_ctor: typing.Callable[['_RecordDesc', StorageSlot, int], T],
		size: int,
		copy_actions: list[CopyAction],
		props: dict[str, tuple[TypeDesc, int]],
	):
		TypeDesc.__init__(self, size, copy_actions)
		self.view_ctor = view_ctor
		self.props = props

		it = list(props.items())
		it.sort(key=lambda x: x[0])
		self.hsh = hash((('_RecordDesc', self.size), *it))

	def get(self, slot: StorageSlot, off: int) -> T:
		return self.view_ctor(self, slot, off)

	def set(self, slot: StorageSlot, off: int, val: T) -> None:
		assert val.__type_desc__ == self, f'Is right a storage type? {type(val)}'
		actions_apply_copy(self.copy_actions, slot, off, val._storage_slot, val._off)

	def __eq__(self, r):
		if not isinstance(r, _RecordDesc):
			return False
		if r is self:
			return True
		if r.hsh != self.hsh:
			return False
		return self.size == r.size and self.props == r.props

	def __hash__(self):
		return self.hsh


class WithRecordStorageSlot(_WithStorageSlot, typing.Protocol):
	__type_desc__: _RecordDesc
