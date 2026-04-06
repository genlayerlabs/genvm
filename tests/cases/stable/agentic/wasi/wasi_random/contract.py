# { "Depends": "py-genlayer:test" }
import genlayer as gl
import os
import random


class Contract(gl.contract.Contract):
	def __init__(self):
		res: list[str] = []

		rand_bytes = os.urandom(16)
		res.append(f'urandom_hex={rand_bytes.hex()}')

		val = random.randint(0, 2**64)
		res.append(f'random_int={val}')

		val2 = random.random()
		res.append(f'random_float={val2}')

		for i in range(5):
			b = os.urandom(8)
			res.append(f'urandom_seq_{i}={b.hex()}')

		gl.vm.UserError.immediate(res)
