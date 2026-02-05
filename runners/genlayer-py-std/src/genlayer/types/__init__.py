"""
Core type definitions for GenLayer contracts.

This module provides:
- Integer type aliases: u8, u16, ..., u256 (unsigned) and i8, i16, ..., i256 (signed)
- ``bigint``: Alias for int, used to mark performance-critical big integers
- ``Lazy[T]``: Base class for lazy evaluation
- ``Address``: GenLayer address type
- ``Keccak256``: Keccak256 hashing utility
"""

__all__ = (
	# Unsigned integers
	'u8',
	'u16',
	'u24',
	'u32',
	'u40',
	'u48',
	'u56',
	'u64',
	'u72',
	'u80',
	'u88',
	'u96',
	'u104',
	'u112',
	'u120',
	'u128',
	'u136',
	'u144',
	'u152',
	'u160',
	'u168',
	'u176',
	'u184',
	'u192',
	'u200',
	'u208',
	'u216',
	'u224',
	'u232',
	'u240',
	'u248',
	'u256',
	# Signed integers
	'i8',
	'i16',
	'i24',
	'i32',
	'i40',
	'i48',
	'i56',
	'i64',
	'i72',
	'i80',
	'i88',
	'i96',
	'i104',
	'i112',
	'i120',
	'i128',
	'i136',
	'i144',
	'i152',
	'i160',
	'i168',
	'i176',
	'i184',
	'i192',
	'i200',
	'i208',
	'i216',
	'i224',
	'i232',
	'i240',
	'i248',
	'i256',
	# Other types
	'bigint',
	'Lazy',
	'Address',
	'SizedArray',
	'Keccak256',
)

import base64
import typing
import collections.abc

from .keccak import Keccak256


u8 = typing.NewType('u8', int)
u16 = typing.NewType('u16', int)
u24 = typing.NewType('u24', int)
u32 = typing.NewType('u32', int)
u40 = typing.NewType('u40', int)
u48 = typing.NewType('u48', int)
u56 = typing.NewType('u56', int)
u64 = typing.NewType('u64', int)
u72 = typing.NewType('u72', int)
u80 = typing.NewType('u80', int)
u88 = typing.NewType('u88', int)
u96 = typing.NewType('u96', int)
u104 = typing.NewType('u104', int)
u112 = typing.NewType('u112', int)
u120 = typing.NewType('u120', int)
u128 = typing.NewType('u128', int)
u136 = typing.NewType('u136', int)
u144 = typing.NewType('u144', int)
u152 = typing.NewType('u152', int)
u160 = typing.NewType('u160', int)
u168 = typing.NewType('u168', int)
u176 = typing.NewType('u176', int)
u184 = typing.NewType('u184', int)
u192 = typing.NewType('u192', int)
u200 = typing.NewType('u200', int)
u208 = typing.NewType('u208', int)
u216 = typing.NewType('u216', int)
u224 = typing.NewType('u224', int)
u232 = typing.NewType('u232', int)
u240 = typing.NewType('u240', int)
u248 = typing.NewType('u248', int)
u256 = typing.NewType('u256', int)
"""
Alias for int that is used for typing
"""

i8 = typing.NewType('i8', int)
i16 = typing.NewType('i16', int)
i24 = typing.NewType('i24', int)
i32 = typing.NewType('i32', int)
i40 = typing.NewType('i40', int)
i48 = typing.NewType('i48', int)
i56 = typing.NewType('i56', int)
i64 = typing.NewType('i64', int)
i72 = typing.NewType('i72', int)
i80 = typing.NewType('i80', int)
i88 = typing.NewType('i88', int)
i96 = typing.NewType('i96', int)
i104 = typing.NewType('i104', int)
i112 = typing.NewType('i112', int)
i120 = typing.NewType('i120', int)
i128 = typing.NewType('i128', int)
i136 = typing.NewType('i136', int)
i144 = typing.NewType('i144', int)
i152 = typing.NewType('i152', int)
i160 = typing.NewType('i160', int)
i168 = typing.NewType('i168', int)
i176 = typing.NewType('i176', int)
i184 = typing.NewType('i184', int)
i192 = typing.NewType('i192', int)
i200 = typing.NewType('i200', int)
i208 = typing.NewType('i208', int)
i216 = typing.NewType('i216', int)
i224 = typing.NewType('i224', int)
i232 = typing.NewType('i232', int)
i240 = typing.NewType('i240', int)
i248 = typing.NewType('i248', int)
i256 = typing.NewType('i256', int)

bigint = typing.NewType('bigint', int)
"""
Just an alias for :py:class:`int`, it is introduced to prevent accidental use of low-performance big integers in the store
"""


class Lazy[T]:
	"""
	Base class to support lazy evaluation
	"""

	__slots__ = ('_eval', '_exc', '_res')

	_eval: typing.Callable[[], T] | None
	_exc: Exception | None
	_res: T | None

	def __init__(self, _eval: typing.Callable[[], T]):
		self._eval = _eval
		self._exc = None
		self._res = None

	def get(self) -> T:
		"""
		Performs evaluation if necessary (only ones) and stores the result

		:returns: result of evaluating
		:raises: *iff* evaluation raised, this outcome is also cached, so subsequent calls will raise same exception
		"""
		if self._eval is not None:
			ev = self._eval
			self._eval = None
			try:
				self._res = ev()
			except Exception as e:
				self._exc = e
		if self._exc is not None:
			raise self._exc
		return self._res  # type: ignore


class Address:
	"""
	Represents GenLayer Address
	"""

	SIZE: typing.Final[int] = 20
	"""
	Constant that represents size of a Genlayer address
	"""

	ZERO: typing.ClassVar['Address']
	"""
	The zero address (0x0000000000000000000000000000000000000000)
	"""

	__slots__ = ('_as_bytes', '_as_hex')

	_as_bytes: bytes
	_as_hex: str | None

	def __init__(self, val: 'str | collections.abc.Buffer | Address'):
		"""
		:param val: either a hex encoded address (that starts with '0x'), or base64 encoded address, or buffer of 20 bytes

		.. warning::
			checksum validation is not performed
		"""
		self._as_hex = None
		if isinstance(val, Address):
			self._as_bytes = val.as_bytes
			return
		if isinstance(val, str):
			if len(val) == 2 + Address.SIZE * 2 and val.startswith('0x'):
				val = bytes.fromhex(val[2:])
			elif len(val) > Address.SIZE:
				val = base64.b64decode(val)
		else:
			val = bytes(val)
		if not isinstance(val, bytes) or len(val) != Address.SIZE:
			raise Exception(f'invalid address {val}')
		self._as_bytes = val

	@property
	def as_bytes(self) -> bytes:
		"""
		>>> Address('0x5b38da6a701c568545dcfcb03fcb875f56beddc4').as_bytes
		b'[8\\xdajp\\x1cV\\x85E\\xdc\\xfc\\xb0?\\xcb\\x87_V\\xbe\\xdd\\xc4'

		:returns: raw bytes of an address (most compact representation)
		"""
		return self._as_bytes

	@property
	def as_hex(self) -> str:
		"""
		>>> Address('0x5b38da6a701c568545dcfcb03fcb875f56beddc4').as_hex
		'0x5B38Da6a701c568545dCfcB03FcB875f56beddC4'

		:returns: checksum string representation
		"""
		if self._as_hex is None:
			simple = self._as_bytes.hex()
			hasher = Keccak256()
			hasher.update(simple.encode('ascii'))
			low_up = hasher.digest().hex()
			res = ['0', 'x']
			for i in range(len(simple)):
				if low_up[i] in ['0', '1', '2', '3', '4', '5', '6', '7']:
					res.append(simple[i])
				else:
					res.append(simple[i].upper())
			self._as_hex = ''.join(res)
		return self._as_hex

	@property
	def as_b64(self) -> str:
		"""
		>>> Address('0x5b38da6a701c568545dcfcb03fcb875f56beddc4').as_b64
		'WzjaanAcVoVF3PywP8uHX1a+3cQ='

		:returns: base64 representation of an address (most compact string)
		"""
		return str(base64.b64encode(self.as_bytes), encoding='ascii')

	@property
	def as_int(self) -> u160:
		"""
		>>> Address('0x5b38da6a701c568545dcfcb03fcb875f56beddc4').as_int
		1123907236495940146162314350759402901750813440091
		>>> hex(Address('0x5b38da6a701c568545dcfcb03fcb875f56beddc4').as_int)
		'0xc4ddbe565f87cb3fb0fcdc4585561c706ada385b'


		:returns: int representation of an address (unsigned little endian)
		"""
		return u160(int.from_bytes(self._as_bytes, 'little', signed=False))

	def __hash__(self):
		return hash(self._as_bytes)

	def __lt__(self, r):
		assert isinstance(r, Address)
		return self._as_bytes < r._as_bytes

	def __le__(self, r):
		assert isinstance(r, Address)
		return self._as_bytes <= r._as_bytes

	def __eq__(self, r):
		if not isinstance(r, Address):
			return False
		return self._as_bytes == r._as_bytes

	def __ge__(self, r):
		assert isinstance(r, Address)
		return self._as_bytes >= r._as_bytes

	def __gt__(self, r):
		assert isinstance(r, Address)
		return self._as_bytes > r._as_bytes

	def __repr__(self) -> str:
		return 'Address("' + self.as_hex + '")'

	def __str__(self) -> str:
		return self.as_hex

	def __format__(self, fmt: typing.Literal['x', 'b64', 'cd', '']) -> str:  # type: ignore
		match fmt:
			case 's':
				return self.__str__()
			case 'x':
				return self.as_hex
			case 'b64':
				return self.as_b64
			case 'cd':
				return 'addr#' + ''.join(['{:02x}'.format(x) for x in self._as_bytes])
			case '':
				return repr(self)
			case fmt:
				raise TypeError(f'unsupported format {fmt!r}')


Address.ZERO = Address(b'\x00' * 20)


class SizedArray[T, S: int](typing.Protocol):
	def __len__(self) -> int: ...
	def __getitem__(self, index: typing.SupportsIndex, /) -> T: ...
	def __iter__(self) -> typing.Iterator[T]: ...
