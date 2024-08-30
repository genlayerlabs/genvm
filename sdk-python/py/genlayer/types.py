import base64
import typing
import abc

class Address(bytes):
	SIZE = 32

	def __new__(cls, val: str | bytes):
		if isinstance(val, str) or len(val) > Address.SIZE:
			val = base64.b64decode(val)
		if len(val) != Address.SIZE:
			raise Exception("invalid address")
		return bytes.__new__(cls, val)

i8 = typing.NewType('i8', int)
i64 = typing.NewType('i64', int)
u32 = typing.NewType('u32', int)
u64 = typing.NewType('u64', int)
