import typing

from .core import *
from .core import _WithStorageSlot

import genlayer.py._internal.reflect as reflect


class RecordExtraFields(_WithStorageSlot, typing.Protocol):
	__type_desc__: '_RecordDesc'


class _RecordDesc[T: RecordExtraFields](TypeDesc):
	props: dict[str, tuple[TypeDesc, int]]

	__slots__ = ('props', 'hsh', 'cls')

	def __init__(
		self,
		size: int,
		copy_actions: list[CopyAction],
		props: dict[str, tuple[TypeDesc, int]],
		cls: typing.Type[T],
	):
		TypeDesc.__init__(self, size, copy_actions)
		self.props = props
		self.cls = cls

		it = list(props.items())
		it.sort(key=lambda x: x[0])
		self.hsh = hash((('_RecordDesc', self.size), *it))

	def get(self, slot: Slot, off: int) -> T:
		slf = self.cls.__new__(self.cls)
		slf._storage_slot = slot
		slf._off = off
		slf.__type_desc__ = self
		return slf

	def set(self, slot: Slot, off: int, val: T) -> None:
		assert hasattr(
			val, '__type_desc__'
		), f'Is right the same storage type? `{reflect.repr_type(self.cls)}` <- `{reflect.repr_type(type(val))}`'
		assert (
			val.__type_desc__ == self
		), f'Is right the same storage type? `{reflect.repr_type(self.cls)}` <- `{reflect.repr_type(type(val))}`'
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
