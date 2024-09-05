from .types import Address
from typing import Any

TYPE_PINT = 0
TYPE_NINT = 1
TYPE_BYTES = 2
TYPE_STR = 3
TYPE_ARR = 4
TYPE_MAP = 5
TYPE_NULL = 6
TYPE_ADDR = 7

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
			append_uleb128(TYPE_NULL)
		elif isinstance(b, int):
			if b >= 0:
				b = (b << 3) | TYPE_PINT
				append_uleb128(b)
			else:
				b = -b - 1
				b = (b << 3) | TYPE_NINT
				append_uleb128(b)
		elif isinstance(b, Address):
			append_uleb128(TYPE_ADDR)
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

def decode(mem: bytes | memoryview) -> Any:
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
		code = code >> 3
		if typ == TYPE_NULL:
			return None
		elif typ == TYPE_PINT:
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
		elif typ == TYPE_ADDR:
			assert code == 0
			ret = mem[:Address.SIZE]
			mem = mem[Address.SIZE:]
			return Address(ret)
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
	return impl()
