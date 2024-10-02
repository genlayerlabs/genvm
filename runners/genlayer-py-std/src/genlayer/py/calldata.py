from .types import Address
from typing import Any

BITS_IN_TYPE = 3

TYPE_SPECIAL = 0
TYPE_PINT = 1
TYPE_NINT = 2
TYPE_BYTES = 3
TYPE_STR = 4
TYPE_ARR = 5
TYPE_MAP = 6

SPECIAL_NULL = (0 << BITS_IN_TYPE) | TYPE_SPECIAL
SPECIAL_TRUE = (1 << BITS_IN_TYPE) | TYPE_SPECIAL
SPECIAL_FALSE = (2 << BITS_IN_TYPE) | TYPE_SPECIAL
SPECIAL_ADDR = (3 << BITS_IN_TYPE) | TYPE_SPECIAL

def encode(x: Any) -> bytes:
	mem = bytearray()
	def append_uleb128(i):
		assert i >= 0
		if i == 0:
			mem.append(0)
		while i > 0:
			cur = i & 0x7f
			i = i >> 7
			if i > 0:
				cur |= 0x80
			mem.append(cur)
	def impl(b):
		if b is None:
			mem.append(SPECIAL_NULL)
		elif b is True:
			mem.append(SPECIAL_TRUE)
		elif b is False:
			mem.append(SPECIAL_FALSE)
		elif isinstance(b, int):
			if b >= 0:
				b = (b << 3) | TYPE_PINT
				append_uleb128(b)
			else:
				b = -b - 1
				b = (b << 3) | TYPE_NINT
				append_uleb128(b)
		elif isinstance(b, Address):
			mem.append(SPECIAL_ADDR)
			mem.extend(b.as_bytes)
		elif isinstance(b, bytes):
			lb = len(b)
			lb = (lb << 3) | TYPE_BYTES
			append_uleb128(lb)
			mem.extend(b)
		elif isinstance(b, str):
			b = b.encode('utf-8')
			lb = len(b)
			lb = (lb << 3) | TYPE_STR
			append_uleb128(lb)
			mem.extend(b)
		elif isinstance(b, (list, tuple)):
			lb = len(b)
			lb = (lb << 3) | TYPE_ARR
			append_uleb128(lb)
			for x in b:
				impl(x)
		elif isinstance(b, dict):
			keys = list(b.keys())
			keys.sort()
			le = len(keys)
			le = (le << 3) | TYPE_MAP
			append_uleb128(le)
			for k in keys:
				if not isinstance(k, str):
					raise Exception(f'key is not string {type(k)}')
				bts = k.encode('utf-8')
				append_uleb128(len(bts))
				mem.extend(bts)
				impl(b[k])
		else:
			raise Exception(f'invalid type {type(b)}')
	impl(x)
	return bytes(mem)

def decode(mem: bytes | memoryview) -> Any: # type: ignore
	mem: memoryview = memoryview(mem)
	def read_uleb128() -> int:
		nonlocal mem
		ret = 0
		off = 0
		while True:
			m = mem[0]
			ret = ret | ((m & 0x7f) << off)
			off += 7
			mem = mem[1:]
			if (m & 0x80) == 0:
				break
		return ret
	def impl() -> Any:
		nonlocal mem
		code = read_uleb128()
		typ = code & 0x7
		if typ == TYPE_SPECIAL:
			if code == SPECIAL_NULL:
				return None
			if code == SPECIAL_FALSE:
				return False
			if code == SPECIAL_TRUE:
				return True
			if code == SPECIAL_ADDR:
				ret = mem[:Address.SIZE]
				mem = mem[Address.SIZE:]
				return Address(ret)
			raise Exception(f"Unknown special {bin(code)} {hex(code)}")
		code = code >> 3
		if typ == TYPE_PINT:
			return code
		elif typ == TYPE_NINT:
			return -code - 1
		elif typ == TYPE_BYTES:
			ret = mem[:code]
			mem = mem[code:]
			return ret
		elif typ == TYPE_STR:
			ret = mem[:code]
			mem = mem[code:]
			return str(ret, encoding='utf-8')
		elif typ == TYPE_ARR:
			ret = []
			for i in range(code):
				ret.append(impl())
			return ret
		elif typ == TYPE_MAP:
			ret = {}
			prev = None
			for i in range(code):
				le = read_uleb128()
				key = mem[:le]
				mem = mem[le:]
				key = str(key, encoding='utf-8')
				if prev is not None:
					assert prev < key
				prev = key
				assert key not in ret
				ret[key] = impl()
			return ret
		raise Exception(f'invalid type {typ}')
	res = impl()
	if len(mem) != 0:
		raise Exception(f'unparsed end {bytes(mem[:5])}...')
	return res
