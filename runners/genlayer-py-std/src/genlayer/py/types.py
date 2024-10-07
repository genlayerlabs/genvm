import base64
import typing
import abc


class Address:
	SIZE = 32
	_as_bytes: bytes

	def __init__(self, val: str | bytes | memoryview):
		if isinstance(val, memoryview):
			val = bytes(val)
		if isinstance(val, str) or len(val) > Address.SIZE:
			val = base64.b64decode(val)
		if len(val) != Address.SIZE:
			raise Exception('invalid address')
		self._as_bytes = val

	@property
	def as_bytes(self) -> bytes:
		return self._as_bytes

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
		return 'addr:[' + ''.join(['{:02x}'.format(x) for x in self._as_bytes]) + ']'


i8 = typing.NewType('i8', int)
i64 = typing.NewType('i64', int)
u32 = typing.NewType('u32', int)
u64 = typing.NewType('u64', int)


class Rollback(Exception):
	def __init__(self, msg: str):
		self.msg = msg
		super()
