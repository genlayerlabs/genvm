class SameOp:
	def __init__(self, l, r):
		self.l = l
		self.r = r

	def __call__(self, foo, *, void=False):
		threwl = False
		threwr = False
		resl = None
		resr = None
		try:
			resl = foo(self.l)
		except:
			threwl = True
		try:
			resr = foo(self.r)
		except:
			threwr = True
		assert threwl == threwr
		if threwl:
			return
		if void:
			resl = None
			resr = None
		assert resl == resr


import random


def byte_range(first, last):
	return list(range(first, last + 1))


first_values = byte_range(0x00, 0x7F) + byte_range(0xC2, 0xF4)
trailing_values = byte_range(0x80, 0xBF)


def random_utf8_codepoint() -> bytes:
	first = random.choice(first_values)
	if first <= 0x7F:
		return bytes([first])
	elif first <= 0xDF:
		return bytes([first, random.choice(trailing_values)])
	elif first == 0xE0:
		return bytes(
			[first, random.choice(byte_range(0xA0, 0xBF)), random.choice(trailing_values)]
		)
	elif first == 0xED:
		return bytes(
			[first, random.choice(byte_range(0x80, 0x9F)), random.choice(trailing_values)]
		)
	elif first <= 0xEF:
		return bytes(
			[first, random.choice(trailing_values), random.choice(trailing_values)]
		)
	elif first == 0xF0:
		return bytes(
			[
				first,
				random.choice(byte_range(0x90, 0xBF)),
				random.choice(trailing_values),
				random.choice(trailing_values),
			]
		)
	elif first <= 0xF3:
		return bytes(
			[
				first,
				random.choice(trailing_values),
				random.choice(trailing_values),
				random.choice(trailing_values),
			]
		)
	elif first == 0xF4:
		return bytes(
			[
				first,
				random.choice(byte_range(0x80, 0x8F)),
				random.choice(trailing_values),
				random.choice(trailing_values),
			]
		)
	raise Exception('unreachable')


def random_str(size):
	mem = bytearray()
	for x in range(size):
		mem.extend(random_utf8_codepoint())
	return str(mem, encoding='utf-8')
