from genlayer import *
from genlayer.py.storage._internal.generate import storage


@storage
class UserStorage:
	m: TreeMap[str, u32]


def test_compiles():
	pass


from .common import *
import itertools
import random


def dump(v, x, ind=0):
	if x == 0:
		print(f'{" "*ind}null', end='')
		return
	p = v.slots[x - 1]
	print(f'{" "*ind}[key={p.key}, idx={x}, bal={p.balance}]: {{')
	dump(v, p.left, ind + 1)
	print()
	dump(v, p.right, ind + 1)
	print(f'\n{" "*ind}}}', end='')


def verify_invariants(v: TreeMap[str, u32]):
	def check(idx):
		if idx == 0:
			return {'depth': 0}
		cur = v._slots[idx - 1]
		ld = check(cur.left)
		rd = check(cur.right)
		if cur.left != 0:
			assert v._slots[cur.left - 1].key < cur.key
		if cur.right != 0:
			assert cur.key < v._slots[cur.right - 1].key
		ldepth = ld['depth']
		rdepth = rd['depth']
		assert rdepth - ldepth == cur.balance, 'invariant broken'
		return {'depth': 1 + max(ldepth, rdepth)}

	check(v._root)


def test_insert():
	stor = UserStorage()
	dic = {}

	def same_iter():
		for (lk, lv), (rk, rv) in itertools.zip_longest(
			stor.m.items(), sorted(dic.items())
		):
			assert lk == rk
			assert lv == rv

	op = SameOp(stor.m, dic)
	same_iter()
	vals = list(range(1048)) * 2
	vals_dup = vals.copy()
	random.seed(0)
	random.shuffle(vals)
	random.shuffle(vals_dup)
	iteration = 0
	while len(vals) > 0:
		iteration += 1
		it = vals.pop()
		op(len)
		op(lambda x: x[it])
		op(lambda x: x.get(str(it), None))
		op(lambda x: x.__setitem__(str(it), iteration), void=True)
		same_iter()
		verify_invariants(stor.m)

	try:
		op(lambda x: x.__delitem__('-1'), void=True)
		same_iter()
		verify_invariants(stor.m)
	except Exception:
		print('====')
		for l, r in itertools.zip_longest(stor.m.items(), sorted(dic.items())):
			print(l, ' vs ', r)
		raise

	vals = vals_dup
	iteration = 0
	while len(vals) > 0:
		iteration += 1
		it = vals.pop()
		# print(f'iteration {iteration}')
		try:
			op(lambda x: x.__delitem__(str(it)), void=True)
			same_iter()
			verify_invariants(stor.m)
		except Exception:
			print('====')
			for l, r in itertools.zip_longest(stor.m.items(), sorted(dic.items())):
				print(l, ' vs ', r)
			raise
