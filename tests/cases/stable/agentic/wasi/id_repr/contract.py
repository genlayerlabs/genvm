# { "Depends": "py-genlayer:test" }
from genlayer import *


class Dummy:
	pass


class Contract(gl.Contract):
	def __init__(self):
		# Test 1: id() of objects
		obj1 = Dummy()
		obj2 = Dummy()
		print(f'id_obj1={id(obj1)}')
		print(f'id_obj2={id(obj2)}')
		print(f'id_diff={id(obj2) - id(obj1)}')

		# Test 2: Default repr (contains memory address)
		print(f'repr_obj1={repr(obj1)}')
		print(f'repr_obj2={repr(obj2)}')

		# Test 3: id() of common objects
		print(f'id_none={id(None)}')
		print(f'id_true={id(True)}')
		print(f'id_false={id(False)}')
		print(f'id_zero={id(0)}')
		print(f'id_one={id(1)}')

		# Test 4: id() of string interning
		s1 = 'hello'
		s2 = 'hello'
		print(f'string_interned={id(s1) == id(s2)}')
		s3 = 'hello world this is a longer string'
		s4 = 'hello world this is a longer string'
		print(f'long_string_interned={id(s3) == id(s4)}')

		# Test 5: id() used in sorting (dangerous pattern)
		objects = [Dummy() for _ in range(5)]
		sorted_by_id = sorted(objects, key=id)
		ids = [id(o) for o in sorted_by_id]
		# Check if sorting by id gives consistent relative order
		diffs = [ids[i + 1] - ids[i] for i in range(len(ids) - 1)]
		print(f'id_sort_diffs={diffs}')

		# Test 6: object.__hash__ (based on id in CPython)
		hashes = [hash(Dummy()) for _ in range(5)]
		print(f'object_hashes={hashes}')
