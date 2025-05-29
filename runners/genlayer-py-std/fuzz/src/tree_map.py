#!/usr/bin/env python3

from genlayer import *
from genlayer.py.storage import inmem_allocate
import itertools

import sys
from pathlib import Path

sys.path.append(str(Path(__file__).parent.parent))
from fuzz_common import do_fuzzing, StopFuzzingException, FuzzerBuilder


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


def tree_map(buf):
	builder = FuzzerBuilder(buf)

	try:
		iterations = int.from_bytes(builder.fetch(2))

		etalon = {}
		testing = inmem_allocate(TreeMap[str, u32])

		for i in range(iterations):
			op = builder.fetch(1)[0] % 2
			if op == 0:
				key = builder.fetch_str()
				val = u32(int.from_bytes(builder.fetch(4)))
				etalon[key] = val
				testing[key] = val
			elif op == 1:
				key = builder.fetch_str()
				e = etalon.pop(key, None)
				t = testing.pop(key, None)
				assert e == t
			else:
				assert False

		verify_invariants(testing)

		for (lk, lv), (rk, rv) in itertools.zip_longest(
			testing.items(), sorted(etalon.items())
		):
			assert lk == rk
			assert lv == rv
	except StopFuzzingException:
		return


if __name__ == '__main__':
	do_fuzzing(tree_map)
