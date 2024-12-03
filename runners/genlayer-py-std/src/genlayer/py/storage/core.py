from genlayer.py.types import Address
import abc
import collections.abc

import typing
import hashlib

from genlayer.py.types import u256


def _calculate_indirection_addr(l: u256, r: int) -> u256:
	hasher = hashlib.sha3_256()
	hasher.update(l.to_bytes(32, 'little'))
	hasher.update(r.to_bytes(4, 'little'))
	res = hasher.digest()
	return u256(int.from_bytes(res, 'little'))


class StorageMan(typing.Protocol):
	@abc.abstractmethod
	def get_store_slot(self, addr: u256) -> 'StorageSlot':
		pass


class StorageSlot:
	manager: StorageMan

	def __init__(self, addr: u256, manager: StorageMan):
		self.addr = addr
		self.manager = manager

	def indirect(self, off: int) -> 'StorageSlot':
		addr = _calculate_indirection_addr(self.addr, off)
		return self.manager.get_store_slot(addr)

	@abc.abstractmethod
	def read(self, off: int, len: int) -> bytes: ...

	@abc.abstractmethod
	def write(self, off: int, what: collections.abc.Buffer) -> None: ...


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
	"""
	Basic type description
	"""

	size: int
	"""
	size that value takes in current slot
	"""

	copy_actions: list[CopyAction]
	"""
	actions that must be executed for copying this data

	:py:type:`int` represents ``memcpy``
	"""
	alias_to: typing.Any

	def __init__(self, size: int, copy_actions: list[CopyAction]):
		self.copy_actions = copy_actions
		self.size = size
		self.alias_to = None

	@abc.abstractmethod
	def get(self, slot: StorageSlot, off: int) -> T:
		"""
		Method that reads value from slot and offset pair
		"""
		...

	@abc.abstractmethod
	def set(self, slot: StorageSlot, off: int, val: T) -> None:
		"""
		Method that writes value to slot and offset pair
		"""
		...

	def __repr__(self):
		ret: list[str] = []
		if self.alias_to is not None:
			ret.append(repr(self.alias_to))
			ret.append('((')
		ret.append(type(self).__name__)
		ret.append('[')
		for k, v in self.__dict__.items():
			if k == 'alias_to':
				continue
			ret.append(f' {k!r}: {v!r} ;')
		ret.append(']')
		if self.alias_to is not None:
			ret.append('))')
		return ''.join(ret)


class WithStorageSlot(typing.Protocol):
	_storage_slot: StorageSlot
	_off: int


class _FakeStorageSlot(StorageSlot):
	"""
	In-memory storage slot which can be used to create storage entities without "Host"
	"""

	_mem: bytearray

	def __init__(self, addr: u256, manager: StorageMan):
		StorageSlot.__init__(self, addr, manager)
		self._mem = bytearray()

	def read(self, addr: int, le: int) -> bytes:
		self._mem.extend(b'\x00' * (addr + le - len(self._mem)))
		return bytes(memoryview(self._mem)[addr : addr + le])

	def write(self, addr: int, what: memoryview) -> None:
		l = len(what)
		self._mem.extend(b'\x00' * (addr + l - len(self._mem)))
		memoryview(self._mem)[addr : addr + l] = what


class _FakeStorageMan(StorageMan):
	_parts: dict[u256, _FakeStorageSlot]

	def __init__(self):
		self._parts = {}

	def get_store_slot(self, addr: u256) -> StorageSlot:
		return self._parts.setdefault(addr, _FakeStorageSlot(addr, self))

	def debug(self):
		print('=== fake storage ===')
		for k, v in self._parts.items():
			print(f'{hex(k)}\n\t{v._mem}')


ROOT_STORAGE_ADDRESS = u256(0)
