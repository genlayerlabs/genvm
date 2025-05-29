import abc
import collections.abc

import typing
import hashlib

from genlayer.py.types import u256


class Manager(typing.Protocol):
	def get_store_slot(self, addr: bytes) -> 'Slot': ...


class Slot:
	manager: Manager

	__slots__ = ('manager', 'id', '_indir_cache')

	def __init__(self, addr: bytes, manager: Manager):
		self.id = addr
		self.manager = manager

		self._indir_cache = hashlib.sha3_256(addr)

	def indirect(self, off: int, /) -> 'Slot':
		hasher = self._indir_cache.copy()
		hasher.update(off.to_bytes(4, 'little'))
		return self.manager.get_store_slot(hasher.digest())

	@abc.abstractmethod
	def read(self, off: int, len: int, /) -> bytes:
		raise NotImplementedError()

	@abc.abstractmethod
	def write(self, off: int, what: collections.abc.Buffer, /) -> None:
		raise NotImplementedError()

	def as_int(self) -> u256:
		return u256(int.from_bytes(self.id, 'little', signed=False))

	def __repr__(self):
		return f'{type(self).__name__}({self.id.hex()})'

	def __str__(self):
		return f'Slot({self.id.hex()})'


class ComplexCopyAction(typing.Protocol):
	def copy(self, frm: Slot, frm_off: int, to: Slot, to_off: int) -> int: ...


type CopyAction = int | ComplexCopyAction


def actions_apply_copy(
	copy_actions: list[CopyAction],
	to_stor: Slot,
	to_off: int,
	frm_stor: Slot,
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


class TypeDesc[T](metaclass=abc.ABCMeta):
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

	:py:class:`int` represents ``memcpy``
	"""
	alias_to: typing.Any

	__slots__ = ('size', 'copy_actions', 'alias_to')

	def __init__(self, size: int, copy_actions: list[CopyAction]):
		self.copy_actions = copy_actions
		self.size = size
		self.alias_to = None

	@abc.abstractmethod
	def get(self, slot: Slot, off: int) -> T:
		"""
		Method that reads value from slot and offset pair
		"""
		raise NotImplementedError()

	@abc.abstractmethod
	def set(self, slot: Slot, off: int, val: T) -> None:
		"""
		Method that writes value to slot and offset pair
		"""
		raise NotImplementedError()

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


class _WithStorageSlot(typing.Protocol):
	_storage_slot: Slot
	_off: int


class _WithStorageSlotAndTD(_WithStorageSlot, typing.Protocol):
	_item_desc: TypeDesc


class SpecialTypeDesc(TypeDesc):
	__slots__ = ('item_desc', 'view_ctor')

	def __init__(
		self, item_desc: TypeDesc, view_ctor: typing.Callable[[], _WithStorageSlotAndTD]
	):
		self.item_desc = item_desc
		self.view_ctor = view_ctor

	def get(self, slot: Slot, off: int) -> typing.Any:
		ret = self.view_ctor()
		ret._storage_slot = slot
		ret._off = off
		ret._item_desc = self.item_desc

		return ret

	def __eq__(self, r):
		return type(self) == type(r) and self.item_desc == r.item_desc

	def __hash__(self) -> int:
		return hash((type(self).__qualname__, self.item_desc))


class Indirection[T](_WithStorageSlotAndTD):
	"""
	This class provides ability to save data at its own slot. Occupies 1 byte to prevent collision.
	"""

	__slots__ = ('_storage_slot', '_off', '_item_desc')

	def get(self) -> T:
		return self._item_desc.get(self._storage_slot.indirect(self._off), 0)

	def slot(self) -> Slot:
		"""
		:returns: :py:class:`Slot` at which data resides
		"""
		return self._storage_slot.indirect(self._off)

	def __init__(self):
		raise TypeError('this class can not be instantiated by user')


class IndirectionTypeDesc[T](
	SpecialTypeDesc, TypeDesc[Indirection[T]], ComplexCopyAction
):
	def __init__(self, item_desc: TypeDesc):
		SpecialTypeDesc.__init__(self, item_desc, lambda: Indirection.__new__(Indirection))
		TypeDesc.__init__(self, 1, [self])

	def set(self, slot: Slot, off: int, val: Indirection[T]) -> None:
		self.item_desc.set(slot.indirect(off), 0, val.get())

	def copy(self, frm: Slot, frm_off: int, to: Slot, to_off: int) -> int:
		val = self.item_desc.get(frm.indirect(frm_off), 0)
		self.item_desc.set(to.indirect(to_off), 0, val)

		return 1


class PseudoSequence[T](
	collections.abc.Sized, collections.abc.Iterable[T], typing.Protocol
):
	"""
	Class that supports indexing elements but not slicing
	"""

	def __getitem__(self, key: int, /) -> T: ...


class VLA[T](_WithStorageSlotAndTD, PseudoSequence[T]):
	"""
	Variable Length Array. Can be used in pair with :py:class:`~Indirection` to save length at the same place as data.
	Can also be used in C language way. Occupies at least 4 bytes (for length)
	"""

	_storage_slot: Slot
	_off: int
	_item_desc: TypeDesc[T]

	__slots__ = ('_storage_slot', '_off', '_item_desc')

	def __len__(self) -> int:
		data = self._storage_slot.read(self._off, 4)
		return int.from_bytes(data, byteorder='little')

	def __getitem__(self, idx: int) -> T:
		if idx >= len(self) or idx < 0:
			raise IndexError(f'{idx} out of range 0..{len(self)}')

		return self._item_desc.get(
			self._storage_slot, self._off + 4 + idx * self._item_desc.size
		)

	def __setitem__(self, idx: int, val: T):
		if idx >= len(self) or idx < 0:
			raise IndexError(f'{idx} out of range 0..{len(self)}')

		return self._item_desc.set(
			self._storage_slot, self._off + 4 + idx * self._item_desc.size, val
		)

	def __iter__(self):
		l = len(self)
		for i in range(l):
			yield self[i]

	def append(self, val: T):
		le = len(self)
		self._item_desc.set(
			self._storage_slot, self._off + 4 + le * self._item_desc.size, val
		)
		self._storage_slot.write(self._off, (le + 1).to_bytes(4, 'little'))

	def extend(self, val: PseudoSequence[T]):
		if isinstance(val, bytes):
			from .desc_base_types import _u8_desc

			assert self._item_desc == _u8_desc
			self._storage_slot.write(self._off, len(val).to_bytes(4, 'little'))
			self._storage_slot.write(self._off + 4, val)
			return

		self.truncate()

		for v in val:
			self.append(v)

	def slot(self) -> Slot:
		return self._storage_slot

	def truncate(self, to: int = 0):
		if to > len(self):
			raise IndexError(f'{to} out of range 0..{len(self)}')
		self._storage_slot.write(self._off, to.to_bytes(4, 'little'))


class VLATypeDesc[T](SpecialTypeDesc, TypeDesc[VLA[T]], ComplexCopyAction):
	SIZE = 2**32 - 1

	def __init__(self, item_desc: TypeDesc):
		SpecialTypeDesc.__init__(self, item_desc, lambda: VLA.__new__(VLA))
		TypeDesc.__init__(self, VLATypeDesc.SIZE, [self])

	def set(self, slot: Slot, off: int, val: VLA[T]) -> None:
		self.copy(val._storage_slot, val._off, slot, off)

	def copy(self, frm: Slot, frm_off: int, to: Slot, to_off: int) -> int:
		src: VLA[T] = self.get(frm, frm_off)
		dst: VLA[T] = self.get(to, to_off)

		dst.truncate()
		dst.extend(src)

		return VLATypeDesc.SIZE


class InmemSlot(Slot):
	"""
	In-memory storage slot which can be used to create storage entities without "Host"
	"""

	__slots__ = ('_mem',)

	_mem: bytearray

	def __init__(self, addr: bytes, manager: Manager):
		Slot.__init__(self, addr, manager)
		self._mem = bytearray()

	def read(self, off: int, le: int) -> bytes:
		self._mem.extend(b'\x00' * (off + le - len(self._mem)))
		return bytes(memoryview(self._mem)[off : off + le])

	def write(self, off: int, what: collections.abc.Buffer) -> None:
		what = memoryview(what)
		l = len(what)
		self._mem.extend(b'\x00' * (off + l - len(self._mem)))
		memoryview(self._mem)[off : off + l] = what


class InmemManager(Manager):
	_parts: dict[bytes, InmemSlot]

	__slots__ = ('_parts',)

	def __init__(self):
		self._parts = {}

	def get_store_slot(self, addr: bytes) -> Slot:
		return self._parts.setdefault(addr, InmemSlot(addr, self))

	def debug(self):
		print('=== fake storage ===')
		for k, v in self._parts.items():
			print(f'{k.hex()}\n\t{v._mem}')


ROOT_SLOT_ID: typing.Final = b'\x00' * 32
