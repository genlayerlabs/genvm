from genlayer.py.types import Address
import abc
import collections.abc

import typing
import hashlib


def _calculate_indirection_addr(l: Address, r: int) -> Address:
	hasher = hashlib.sha3_256()
	hasher.update(l.as_bytes)
	hasher.update(r.to_bytes(4, 'little', signed=False))
	res = hasher.digest()
	return Address(res)


class StorageMan(typing.Protocol):
	@abc.abstractmethod
	def get_store_slot(self, addr: Address) -> 'StorageSlot':
		pass


class StorageSlot:
	manager: StorageMan

	def __init__(self, addr: Address, manager: StorageMan):
		self.addr = addr
		self.manager = manager

	def indirect(self, off: int) -> 'StorageSlot':
		addr = _calculate_indirection_addr(self.addr, off)
		return self.manager.get_store_slot(addr)

	@abc.abstractmethod
	def read(self, addr: int, len: int) -> bytes: ...

	@abc.abstractmethod
	def write(self, addr: int, what: collections.abc.Buffer) -> None: ...


class ComplexCopyAction(typing.Protocol):
	@abc.abstractmethod
	def copy(
		self, frm: StorageSlot, frm_off: int, to: StorageSlot, to_off: int
	) -> int: ...


type CopyAction = int | ComplexCopyAction


def actions_apply_copy(
	copy_actions: list[CopyAction],
	to_stor: StorageSlot,
	to_off: int,
	frm_stor: StorageSlot,
	frm_off: int,
) -> int:
	cum_off = 0
	for act in copy_actions:
		if isinstance(act, int):
			to_stor.write(to_off + cum_off, frm_stor.read(frm_off + cum_off, act))
			cum_off += act
		else:
			cum_off += act.copy(frm_stor, frm_off + cum_off, to_stor, to_off + cum_off)
	return cum_off


def actions_append(l: list[CopyAction], r: list[CopyAction]):
	it = iter(r)
	if len(l) > 0 and len(r) > 0 and isinstance(l[-1], int) and isinstance(r[0], int):
		l[-1] += r[0]
		next(it)
	l.extend(it)


class TypeDesc[T]:
	size: int
	copy_actions: list[CopyAction]

	def __init__(self, size: int, copy_actions: list[CopyAction]):
		self.copy_actions = copy_actions
		self.size = size

	@abc.abstractmethod
	def get(self, slot: StorageSlot, off: int) -> T: ...

	@abc.abstractmethod
	def set(self, slot: StorageSlot, off: int, val: T) -> None: ...


class WithStorageSlot(typing.Protocol):
	__description__: typing.ClassVar[TypeDesc]
	_storage_slot: StorageSlot
	_off: int


ROOT_STORAGE_ADDRESS = Address(bytes([0] * 32))
