import typing

from .core import *


class _RecordDesc[T: WithStorageSlot](TypeDesc):
	def __init__(
		self,
		view_ctor: typing.Callable[[StorageSlot, int], T],
		size: int,
		actions: list[CopyAction],
	):
		TypeDesc.__init__(self, size, actions)
		self.view_ctor = view_ctor

	def get(self, slot: StorageSlot, off: int) -> T:
		return self.view_ctor(slot, off)

	def set(self, slot: StorageSlot, off: int, val: T) -> None:
		if self is not val.__description__:
			raise Exception("can't store")
		actions_apply_copy(self.copy_actions, slot, off, val._storage_slot, val._off)
