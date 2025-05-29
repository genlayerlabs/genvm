#!/usr/bin/env python3

import genlayer.py.calldata as calldata

import sys
from pathlib import Path

sys.path.append(str(Path(__file__).parent.parent))
from fuzz_common import do_fuzzing, StopFuzzingException, FuzzerBuilder


def calldata_encoding(buf):
	builder = FuzzerBuilder(buf)

	def create_atom():
		kind = builder.fetch(1)[0] % 6
		if kind == 0:
			return int.from_bytes(builder.fetch(7), signed=True)
		if kind == 1:
			return True
		if kind == 2:
			return False
		if kind == 3:
			return None
		if kind == 4:
			return builder.fetch(builder.fetch(1)[0])
		if kind == 5:
			builder.fetch_str()

	def create_any(depth):
		if depth == 0:
			return create_atom()
		kind = builder.fetch(1)[0] % 3
		if kind == 0:
			return create_atom()
		if kind == 1:
			le = builder.fetch(1)[0]
			lst = []
			for i in range(le):
				lst.append(create_any(depth - 1))
			return lst
		if kind == 1:
			le = builder.fetch(1)[0]
			lst = {}
			for i in range(le):
				k = builder.fetch_str()
				v = create_any(depth - 1)
				lst[k] = v
			return lst

	try:
		depth = builder.fetch(1)[0] % 5
		created_obj = create_any(depth)
	except StopFuzzingException:
		return

	encoded = '<unbound>'
	decoded = '<unbound>'
	try:
		encoded = calldata.encode(created_obj)
		decoded = calldata.decode(encoded)

		assert created_obj == decoded
	except Exception as e:
		raise Exception(
			f'failed base={created_obj} decoded={decoded} encoded={encoded}'
		) from e


if __name__ == '__main__':
	do_fuzzing(calldata_encoding)
