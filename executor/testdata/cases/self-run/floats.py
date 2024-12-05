# { "Depends": "py-genlayer:test" }
import math

import operator

data: list[float] = [
	math.nan,
	math.inf,
	-math.inf,
	1.41421356237,
	-1.41421356237,
	1.0,
	-1,
	1e5,
	-1e5,
	0.0,
	-0.0,
]

import struct
import numpy as np

for x in data:
	print('new x')
	for y in data:
		for op in [math.floor, math.ceil, math.sqrt, math.sin, int, math.exp]:
			try:
				res = op(x)
				if math.isnan(res):
					res = math.nan
				s = struct.pack('>d', res)
				print(f'\t{x:.5f} {op} {res:.5f} {s.hex()}')
			except:
				pass
		print('\tnew y')
		for op in [
			operator.add,
			operator.sub,
			operator.mul,
			operator.truediv,
		]:  # operator.pow
			try:
				res = op(x, y)
				if math.isnan(res):
					res = math.nan
				s = struct.pack('>d', res)
				print(f'\t\t{x:.5f} {op} {y:.5f} {res:.5f} {s.hex()}')
			except:
				pass

exit(0)
