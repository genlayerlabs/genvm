# { "Depends": "py-genlayer:test" }
import genlayer as gl


class Dummy:
	pass


class Contract(gl.contract.Contract):
	def __init__(self):
		res: list[str] = []

		obj1 = Dummy()
		obj2 = Dummy()
		res.append(f'id_obj1={id(obj1)}')
		res.append(f'id_obj2={id(obj2)}')
		res.append(f'id_diff={id(obj2) - id(obj1)}')

		res.append(f'repr_obj1={repr(obj1)}')
		res.append(f'repr_obj2={repr(obj2)}')

		res.append(f'id_none={id(None)}')
		res.append(f'id_true={id(True)}')
		res.append(f'id_false={id(False)}')
		res.append(f'id_zero={id(0)}')
		res.append(f'id_one={id(1)}')

		s1 = 'hello'
		s2 = 'hello'
		res.append(f'string_interned={id(s1) == id(s2)}')
		s3 = 'hello world this is a longer string'
		s4 = 'hello world this is a longer string'
		res.append(f'long_string_interned={id(s3) == id(s4)}')

		objects = [Dummy() for _ in range(5)]
		sorted_by_id = sorted(objects, key=id)
		ids = [id(o) for o in sorted_by_id]
		diffs = [ids[i + 1] - ids[i] for i in range(len(ids) - 1)]
		res.append(f'id_sort_diffs={diffs}')

		hashes = [hash(Dummy()) for _ in range(5)]
		res.append(f'object_hashes={hashes}')

		gl.vm.UserError.immediate(res)
