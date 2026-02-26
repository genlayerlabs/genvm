# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	def __init__(self):
		# Test 1: Set iteration order (affected by hash randomization)
		s = {'apple', 'banana', 'cherry', 'date', 'elderberry', 'fig', 'grape'}
		order = list(s)
		print(f'set_order={order}')

		# Test 2: Dict from set-like construction
		d = {k: i for i, k in enumerate(s)}
		print(f'dict_from_set={list(d.keys())}')

		# Test 3: frozenset repr (order may vary)
		fs = frozenset(['x', 'y', 'z', 'w', 'a', 'b'])
		print(f'frozenset_repr={repr(fs)}')

		# Test 4: set operations that produce new sets
		s1 = {'a', 'b', 'c', 'd'}
		s2 = {'c', 'd', 'e', 'f'}
		union = list(s1 | s2)
		print(f'set_union_order={union}')
		inter = list(s1 & s2)
		print(f'set_inter_order={inter}')
		diff = list(s1 - s2)
		print(f'set_diff_order={diff}')

		# Test 5: Large set to amplify ordering differences
		big_set = {f'item_{i}' for i in range(50)}
		big_order = list(big_set)
		print(f'big_set_first_10={big_order[:10]}')
		print(f'big_set_hash={hash(frozenset(big_set))}')

		# Test 6: Dict ordering after deletions and re-insertions
		d2 = {}
		for i in range(20):
			d2[f'key_{i}'] = i
		for i in range(0, 20, 2):
			del d2[f'key_{i}']
		for i in range(20, 30):
			d2[f'key_{i}'] = i
		print(f'dict_keys_after_mutations={list(d2.keys())}')
