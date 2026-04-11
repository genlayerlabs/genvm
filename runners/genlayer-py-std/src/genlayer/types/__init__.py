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
	'KeccakHash',
)

import base64
import typing
import collections.abc

from .keccak import Keccak256, KeccakHash


class StaticIntMeta(typing.NamedTuple):
	size: int
	signed: bool


u8 = typing.Annotated[int, StaticIntMeta(1, False)]
"""
Fixed size integer alias for storage
"""
u16 = typing.Annotated[int, StaticIntMeta(2, False)]
"""
Fixed size integer alias for storage
"""
u24 = typing.Annotated[int, StaticIntMeta(3, False)]
"""
Fixed size integer alias for storage
"""
u32 = typing.Annotated[int, StaticIntMeta(4, False)]
"""
Fixed size integer alias for storage
"""
u40 = typing.Annotated[int, StaticIntMeta(5, False)]
"""
Fixed size integer alias for storage
"""
u48 = typing.Annotated[int, StaticIntMeta(6, False)]
"""
Fixed size integer alias for storage
"""
u56 = typing.Annotated[int, StaticIntMeta(7, False)]
"""
Fixed size integer alias for storage
"""
u64 = typing.Annotated[int, StaticIntMeta(8, False)]
"""
Fixed size integer alias for storage
"""
u72 = typing.Annotated[int, StaticIntMeta(9, False)]
"""
Fixed size integer alias for storage
"""
u80 = typing.Annotated[int, StaticIntMeta(10, False)]
"""
Fixed size integer alias for storage
"""
u88 = typing.Annotated[int, StaticIntMeta(11, False)]
"""
Fixed size integer alias for storage
"""
u96 = typing.Annotated[int, StaticIntMeta(12, False)]
"""
Fixed size integer alias for storage
"""
u104 = typing.Annotated[int, StaticIntMeta(13, False)]
"""
Fixed size integer alias for storage
"""
u112 = typing.Annotated[int, StaticIntMeta(14, False)]
"""
Fixed size integer alias for storage
"""
u120 = typing.Annotated[int, StaticIntMeta(15, False)]
"""
Fixed size integer alias for storage
"""
u128 = typing.Annotated[int, StaticIntMeta(16, False)]
"""
Fixed size integer alias for storage
"""
u136 = typing.Annotated[int, StaticIntMeta(17, False)]
"""
Fixed size integer alias for storage
"""
u144 = typing.Annotated[int, StaticIntMeta(18, False)]
"""
Fixed size integer alias for storage
"""
u152 = typing.Annotated[int, StaticIntMeta(19, False)]
"""
Fixed size integer alias for storage
"""
u160 = typing.Annotated[int, StaticIntMeta(20, False)]
"""
Fixed size integer alias for storage
"""
u168 = typing.Annotated[int, StaticIntMeta(21, False)]
"""
Fixed size integer alias for storage
"""
u176 = typing.Annotated[int, StaticIntMeta(22, False)]
"""
Fixed size integer alias for storage
"""
u184 = typing.Annotated[int, StaticIntMeta(23, False)]
"""
Fixed size integer alias for storage
"""
u192 = typing.Annotated[int, StaticIntMeta(24, False)]
"""
Fixed size integer alias for storage
"""
u200 = typing.Annotated[int, StaticIntMeta(25, False)]
"""
Fixed size integer alias for storage
"""
u208 = typing.Annotated[int, StaticIntMeta(26, False)]
"""
Fixed size integer alias for storage
"""
u216 = typing.Annotated[int, StaticIntMeta(27, False)]
"""
Fixed size integer alias for storage
"""
u224 = typing.Annotated[int, StaticIntMeta(28, False)]
"""
Fixed size integer alias for storage
"""
u232 = typing.Annotated[int, StaticIntMeta(29, False)]
"""
Fixed size integer alias for storage
"""
u240 = typing.Annotated[int, StaticIntMeta(30, False)]
"""
Fixed size integer alias for storage
"""
u248 = typing.Annotated[int, StaticIntMeta(31, False)]
"""
Fixed size integer alias for storage
"""
u256 = typing.Annotated[int, StaticIntMeta(32, False)]
"""
Fixed size integer alias for storage
"""

i8 = typing.Annotated[int, StaticIntMeta(1, True)]
"""
Fixed size integer alias for storage
"""
i16 = typing.Annotated[int, StaticIntMeta(2, True)]
"""
Fixed size integer alias for storage
"""
i24 = typing.Annotated[int, StaticIntMeta(3, True)]
"""
Fixed size integer alias for storage
"""
i32 = typing.Annotated[int, StaticIntMeta(4, True)]
"""
Fixed size integer alias for storage
"""
i40 = typing.Annotated[int, StaticIntMeta(5, True)]
"""
Fixed size integer alias for storage
"""
i48 = typing.Annotated[int, StaticIntMeta(6, True)]
"""
Fixed size integer alias for storage
"""
i56 = typing.Annotated[int, StaticIntMeta(7, True)]
"""
Fixed size integer alias for storage
"""
i64 = typing.Annotated[int, StaticIntMeta(8, True)]
"""
Fixed size integer alias for storage
"""
i72 = typing.Annotated[int, StaticIntMeta(9, True)]
"""
Fixed size integer alias for storage
"""
i80 = typing.Annotated[int, StaticIntMeta(10, True)]
"""
Fixed size integer alias for storage
"""
i88 = typing.Annotated[int, StaticIntMeta(11, True)]
"""
Fixed size integer alias for storage
"""
i96 = typing.Annotated[int, StaticIntMeta(12, True)]
"""
Fixed size integer alias for storage
"""
i104 = typing.Annotated[int, StaticIntMeta(13, True)]
"""
Fixed size integer alias for storage
"""
i112 = typing.Annotated[int, StaticIntMeta(14, True)]
"""
Fixed size integer alias for storage
"""
i120 = typing.Annotated[int, StaticIntMeta(15, True)]
"""
Fixed size integer alias for storage
"""
i128 = typing.Annotated[int, StaticIntMeta(16, True)]
"""
Fixed size integer alias for storage
"""
i136 = typing.Annotated[int, StaticIntMeta(17, True)]
"""
Fixed size integer alias for storage
"""
i144 = typing.Annotated[int, StaticIntMeta(18, True)]
"""
Fixed size integer alias for storage
"""
i152 = typing.Annotated[int, StaticIntMeta(19, True)]
"""
Fixed size integer alias for storage
"""
i160 = typing.Annotated[int, StaticIntMeta(20, True)]
"""
Fixed size integer alias for storage
"""
i168 = typing.Annotated[int, StaticIntMeta(21, True)]
"""
Fixed size integer alias for storage
"""
i176 = typing.Annotated[int, StaticIntMeta(22, True)]
"""
Fixed size integer alias for storage
"""
i184 = typing.Annotated[int, StaticIntMeta(23, True)]
"""
Fixed size integer alias for storage
"""
i192 = typing.Annotated[int, StaticIntMeta(24, True)]
"""
Fixed size integer alias for storage
"""
i200 = typing.Annotated[int, StaticIntMeta(25, True)]
"""
Fixed size integer alias for storage
"""
i208 = typing.Annotated[int, StaticIntMeta(26, True)]
"""
Fixed size integer alias for storage
"""
i216 = typing.Annotated[int, StaticIntMeta(27, True)]
"""
Fixed size integer alias for storage
"""
i224 = typing.Annotated[int, StaticIntMeta(28, True)]
"""
Fixed size integer alias for storage
"""
i232 = typing.Annotated[int, StaticIntMeta(29, True)]
"""
Fixed size integer alias for storage
"""
i240 = typing.Annotated[int, StaticIntMeta(30, True)]
"""
Fixed size integer alias for storage
"""
i248 = typing.Annotated[int, StaticIntMeta(31, True)]
"""
Fixed size integer alias for storage
"""
i256 = typing.Annotated[int, StaticIntMeta(32, True)]
"""
Fixed size integer alias for storage
"""

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

	def __format__(self, fmt: typing.Literal['s', 'x', 'b64', 'cd', '']) -> str:  # type: ignore
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


@typing.runtime_checkable
class SizedArray[T, S: int](typing.Protocol):
	def __len__(self) -> int: ...
	def __getitem__(self, index: typing.SupportsIndex, /) -> T: ...
	def __iter__(self) -> typing.Iterator[T]: ...
