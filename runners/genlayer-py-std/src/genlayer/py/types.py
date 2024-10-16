import base64
import typing
import collections.abc


class Address:
	SIZE = 20
	_as_bytes: bytes

	def __init__(self, val: str | collections.abc.Buffer):
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
		return self._as_bytes

	@property
	def as_b64(self) -> str:
		return str(base64.b64encode(self.as_bytes), encoding='ascii')

	@property
	def as_int(self) -> int:
		return int.from_bytes(self._as_bytes, 'little', signed=False)

	def __hash__(self):
		return hash(self._as_bytes)

	def __eq__(self, r):
		if not isinstance(r, Address):
			return False
		return self._as_bytes == r._as_bytes

	def __repr__(self) -> str:
		return 'addr#' + ''.join(['{:02x}'.format(x) for x in self._as_bytes])


i8 = typing.NewType('i8', int)
i64 = typing.NewType('i64', int)
u32 = typing.NewType('u32', int)
u64 = typing.NewType('u64', int)
u256 = typing.NewType('u256', int)


class Rollback(Exception):
	def __init__(self, msg: str):
		self.msg = msg
		super()
