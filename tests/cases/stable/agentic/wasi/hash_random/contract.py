# { "Depends": "py-genlayer:test" }
import genlayer as gl
import sys


class Contract(gl.contract.Contract):
	def __init__(self):
		res: list[str] = []

		try:
			import os

			seed = os.environ.get('PYTHONHASHSEED', 'NOT_SET')
			res.append(f'PYTHONHASHSEED={seed}')
		except Exception as e:
			res.append(f'PYTHONHASHSEED_error={e}')

		for s in ['hello', 'world', 'test', '', 'a', 'determinism']:
			h = hash(s)
			res.append(f'hash_{s}={h}')

		for b in [b'hello', b'test', b'']:
			h = hash(b)
			res.append(f'hash_bytes_{b}={h}')

		h = hash((1, 2, 'hello'))
		res.append(f'hash_tuple={h}')

		h = hash(frozenset([1, 2, 3]))
		res.append(f'hash_frozenset={h}')

		for i in [0, 1, -1, 42, 2**63]:
			h = hash(i)
			res.append(f'hash_int_{i}={h}')

		res.append(f'hash_algorithm={sys.hash_info.algorithm}')
		res.append(f'hash_bits={sys.hash_info.hash_bits}')
		res.append(f'hash_seed_bits={sys.hash_info.seed_bits}')

		gl.vm.UserError.immediate(res)
