"""
Module that provides aliases for storage types and blockchain-specific types
"""

import base64
import typing
import collections.abc

from .keccak import Keccak256


class Address:
	"""
	Represents GenLayer Address
	"""

	SIZE = 20
	"""
	Constant that represents size of a Genlayer address
	"""

	__slots__ = ('_as_bytes', '_as_hex')

	_as_bytes: bytes
	_as_hex: str | None

	def __init__(self, val: str | collections.abc.Buffer):
		"""
		:param val: either a hex encoded address (that starts with '0x'), or base64 encoded address, or buffer of 20 bytes

		.. warning::
			checksum validation is not performed
		"""
		self._as_hex = None
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
		:returns: raw bytes of an address (most compact representation)
		"""
		return self._as_bytes

	@property
	def as_hex(self) -> str:
		"""
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
		:returns: base64 representation of an address (most compact string)
		"""
		return str(base64.b64encode(self.as_bytes), encoding='ascii')

	@property
	def as_int(self) -> int:
		"""
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
Just an alias for :py:type:`int`, it is introduced to prevent accidental use of low-performance big integers in the store
"""


class Rollback(Exception):
	"""
	Exception that will be treated as a "Rollback"
	"""

	def __init__(self, msg: str):
		self.msg = msg
		super()


class Lazy[T]:
	"""
	Base class to support lazy evaluation
	"""

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
