import pytest

import genlayer.py.calldata as calldata
from genlayer.py.types import Address

import random
from .common import *


def make_rnd(size_limit, rem_depth, rnd: random.Random):
	if rem_depth <= 0:
		r = random.randint(0, 7)
		if r == 0:
			return random.randint(-(10**8), 10**8)
		if r == 1:
			return Address(random.randbytes(Address.SIZE))
		if r == 2:
			return random.randbytes(random.randint(0, size_limit))
		if r == 3:
			return random_str(size_limit)
		if r == 4:
			return True
		if r == 5:
			return False
		return None
	if random.randint(0, 100) % 2 == 0:
		le = random.randint(0, size_limit)
		ret = []
		for i in range(le):
			ret.append(make_rnd(size_limit, rem_depth - 1, rnd))
		return ret
	le = random.randint(0, size_limit)
	ret = {}
	for i in range(le):
		ret[random_str(20)] = make_rnd(size_limit, rem_depth - 1, rnd)
	return ret


def impl_test(size_limit, *, depth, rnd: random.Random):
	import math

	data = make_rnd(size_limit, depth, rnd)
	encoded = calldata.encode(data)
	try:
		assert calldata.decode(encoded) == data
	except:
		print('data=', data, sep='')
		print(f'encoded={encoded}')
		raise


@pytest.mark.parametrize('seed', list(range(100)))
def test_calldata_1(seed):
	impl_test(0, depth=1, rnd=random.Random(seed))


@pytest.mark.parametrize('seed', list(range(100)))
def test_calldata_10_1(seed):
	impl_test(10, depth=1, rnd=random.Random(seed))


@pytest.mark.parametrize('seed', list(range(10)))
def test_calldata_10_3(seed):
	random.seed(seed)
	impl_test(10, depth=3, rnd=random.Random(seed))


@pytest.mark.parametrize('seed', list(range(3)))
def test_calldata_6_8(seed):
	random.seed(seed)
	impl_test(6, depth=8, rnd=random.Random(seed))
