# { "Depends": "py-genlayer:test" }
from genlayer import *
import sys


class Contract(gl.Contract):
	def __init__(self):
		# Test 1: PYTHONHASHSEED value
		try:
			import os

			seed = os.environ.get('PYTHONHASHSEED', 'NOT_SET')
			print(f'PYTHONHASHSEED={seed}')
		except Exception as e:
			print(f'PYTHONHASHSEED_error={e}')

		# Test 2: hash() of strings - affected by hash randomization
		for s in ['hello', 'world', 'test', '', 'a', 'determinism']:
			h = hash(s)
			print(f'hash_{s}={h}')

		# Test 3: hash() of bytes
		for b in [b'hello', b'test', b'']:
			h = hash(b)
			print(f'hash_bytes_{b}={h}')

		# Test 4: hash() of tuples (depends on element hashes)
		h = hash((1, 2, 'hello'))
		print(f'hash_tuple={h}')

		# Test 5: hash() of frozenset (depends on element hashes)
		h = hash(frozenset([1, 2, 3]))
		print(f'hash_frozenset={h}')

		# Test 6: hash() of integers (should be deterministic)
		for i in [0, 1, -1, 42, 2**63]:
			h = hash(i)
			print(f'hash_int_{i}={h}')

		# Test 7: sys.hash_info
		print(f'hash_algorithm={sys.hash_info.algorithm}')
		print(f'hash_bits={sys.hash_info.hash_bits}')
		print(f'hash_seed_bits={sys.hash_info.seed_bits}')
