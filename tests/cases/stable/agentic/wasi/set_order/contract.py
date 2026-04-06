# { "Depends": "py-genlayer:test" }
import genlayer as gl


class Contract(gl.contract.Contract):
	def __init__(self):
		res: list[str] = []

		s = {'apple', 'banana', 'cherry', 'date', 'elderberry', 'fig', 'grape'}
		order = list(s)
		res.append(f'set_order={order}')

		d = {k: i for i, k in enumerate(s)}
		res.append(f'dict_from_set={list(d.keys())}')

		fs = frozenset(['x', 'y', 'z', 'w', 'a', 'b'])
		res.append(f'frozenset_repr={repr(fs)}')

		s1 = {'a', 'b', 'c', 'd'}
		s2 = {'c', 'd', 'e', 'f'}
		union = list(s1 | s2)
		res.append(f'set_union_order={union}')
		inter = list(s1 & s2)
		res.append(f'set_inter_order={inter}')
		diff = list(s1 - s2)
		res.append(f'set_diff_order={diff}')

		big_set = {f'item_{i}' for i in range(50)}
		big_order = list(big_set)
		res.append(f'big_set_first_10={big_order[:10]}')
		res.append(f'big_set_hash={hash(frozenset(big_set))}')

		d2 = {}
		for i in range(20):
			d2[f'key_{i}'] = i
		for i in range(0, 20, 2):
			del d2[f'key_{i}']
		for i in range(20, 30):
			d2[f'key_{i}'] = i
		res.append(f'dict_keys_after_mutations={list(d2.keys())}')

		gl.vm.UserError.immediate(res)
