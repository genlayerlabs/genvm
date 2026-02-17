"""
Core type definitions for GenLayer contracts
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


class StaticIntMeta(typing.NamedTuple):
	size: int
	signed: bool


u8 = typing.Annotated[int, StaticIntMeta(1, False)]
u16 = typing.Annotated[int, StaticIntMeta(2, False)]
u24 = typing.Annotated[int, StaticIntMeta(3, False)]
u32 = typing.Annotated[int, StaticIntMeta(4, False)]
u40 = typing.Annotated[int, StaticIntMeta(5, False)]
u48 = typing.Annotated[int, StaticIntMeta(6, False)]
u56 = typing.Annotated[int, StaticIntMeta(7, False)]
u64 = typing.Annotated[int, StaticIntMeta(8, False)]
u72 = typing.Annotated[int, StaticIntMeta(9, False)]
u80 = typing.Annotated[int, StaticIntMeta(10, False)]
u88 = typing.Annotated[int, StaticIntMeta(11, False)]
u96 = typing.Annotated[int, StaticIntMeta(12, False)]
u104 = typing.Annotated[int, StaticIntMeta(13, False)]
u112 = typing.Annotated[int, StaticIntMeta(14, False)]
u120 = typing.Annotated[int, StaticIntMeta(15, False)]
u128 = typing.Annotated[int, StaticIntMeta(16, False)]
u136 = typing.Annotated[int, StaticIntMeta(17, False)]
u144 = typing.Annotated[int, StaticIntMeta(18, False)]
u152 = typing.Annotated[int, StaticIntMeta(19, False)]
u160 = typing.Annotated[int, StaticIntMeta(20, False)]
u168 = typing.Annotated[int, StaticIntMeta(21, False)]
u176 = typing.Annotated[int, StaticIntMeta(22, False)]
u184 = typing.Annotated[int, StaticIntMeta(23, False)]
u192 = typing.Annotated[int, StaticIntMeta(24, False)]
u200 = typing.Annotated[int, StaticIntMeta(25, False)]
u208 = typing.Annotated[int, StaticIntMeta(26, False)]
u216 = typing.Annotated[int, StaticIntMeta(27, False)]
u224 = typing.Annotated[int, StaticIntMeta(28, False)]
u232 = typing.Annotated[int, StaticIntMeta(29, False)]
u240 = typing.Annotated[int, StaticIntMeta(30, False)]
u248 = typing.Annotated[int, StaticIntMeta(31, False)]
u256 = typing.Annotated[int, StaticIntMeta(32, False)]

i8 = typing.Annotated[int, StaticIntMeta(1, True)]
i16 = typing.Annotated[int, StaticIntMeta(2, True)]
i24 = typing.Annotated[int, StaticIntMeta(3, True)]
i32 = typing.Annotated[int, StaticIntMeta(4, True)]
i40 = typing.Annotated[int, StaticIntMeta(5, True)]
i48 = typing.Annotated[int, StaticIntMeta(6, True)]
i56 = typing.Annotated[int, StaticIntMeta(7, True)]
i64 = typing.Annotated[int, StaticIntMeta(8, True)]
i72 = typing.Annotated[int, StaticIntMeta(9, True)]
i80 = typing.Annotated[int, StaticIntMeta(10, True)]
i88 = typing.Annotated[int, StaticIntMeta(11, True)]
i96 = typing.Annotated[int, StaticIntMeta(12, True)]
i104 = typing.Annotated[int, StaticIntMeta(13, True)]
i112 = typing.Annotated[int, StaticIntMeta(14, True)]
i120 = typing.Annotated[int, StaticIntMeta(15, True)]
i128 = typing.Annotated[int, StaticIntMeta(16, True)]
i136 = typing.Annotated[int, StaticIntMeta(17, True)]
i144 = typing.Annotated[int, StaticIntMeta(18, True)]
i152 = typing.Annotated[int, StaticIntMeta(19, True)]
i160 = typing.Annotated[int, StaticIntMeta(20, True)]
i168 = typing.Annotated[int, StaticIntMeta(21, True)]
i176 = typing.Annotated[int, StaticIntMeta(22, True)]
i184 = typing.Annotated[int, StaticIntMeta(23, True)]
i192 = typing.Annotated[int, StaticIntMeta(24, True)]
i200 = typing.Annotated[int, StaticIntMeta(25, True)]
i208 = typing.Annotated[int, StaticIntMeta(26, True)]
i216 = typing.Annotated[int, StaticIntMeta(27, True)]
i224 = typing.Annotated[int, StaticIntMeta(28, True)]
i232 = typing.Annotated[int, StaticIntMeta(29, True)]
i240 = typing.Annotated[int, StaticIntMeta(30, True)]
i248 = typing.Annotated[int, StaticIntMeta(31, True)]
i256 = typing.Annotated[int, StaticIntMeta(32, True)]

bigint = typing.Annotated[int, 'bigint']
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
			self._as_hex = val.as_hex
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
		return int.from_bytes(self._as_bytes, 'little', signed=False)

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
