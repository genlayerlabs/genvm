# { "Depends": "py-genlayer:test" }
from genlayer import *
import os
import random


class Contract(gl.Contract):
	def __init__(self):
		# Test 1: os.urandom (WASI random_get)
		rand_bytes = os.urandom(16)
		print(f'urandom_hex={rand_bytes.hex()}')

		# Test 2: random module (uses os.urandom for seed)
		val = random.randint(0, 2**64)
		print(f'random_int={val}')

		# Test 3: random.random() float
		val2 = random.random()
		print(f'random_float={val2}')

		# Test 4: Multiple urandom calls to check sequence
		for i in range(5):
			b = os.urandom(8)
			print(f'urandom_seq_{i}={b.hex()}')
