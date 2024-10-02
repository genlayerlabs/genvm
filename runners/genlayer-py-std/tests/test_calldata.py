import genlayer.py.calldata as calldata
from genlayer.py.types import Address

import random
import common

def make_rnd(size_limit, rem_depth):
    if rem_depth <= 0:
        r = random.randint(0, 7)
        if r == 0:
            return random.randint(-10**8, 10**8)
        if r == 1:
            return Address(random.randbytes(32))
        if r == 2:
            return random.randbytes(random.randint(0, size_limit))
        if r == 3:
            return common.random_str(size_limit)
        if r == 4:
            return True
        if r == 5:
            return False
        return None
    if random.randint(0, 100) % 2 == 0:
        le = random.randint(0, size_limit)
        ret = []
        for i in range(le):
            ret.append(make_rnd(size_limit, rem_depth - 1))
        return ret
    le = random.randint(0, size_limit)
    ret = {}
    for i in range(le):
        ret[common.random_str(20)] = make_rnd(size_limit, rem_depth - 1)
    return ret

def impl_test(size_limit, *, depth, repeats=10):
    random.seed(0)
    import math
    for i in range(repeats):
        data = make_rnd(size_limit, depth)
        encoded = calldata.encode(data)
        try:
            assert calldata.decode(encoded) == data
        except:
            print("data=", data, sep='')
            print(f"encoded={encoded}")
            raise

def test_calldata_1():
    impl_test(0, depth=1, repeats=100)

def test_calldata_10_3():
    impl_test(10, depth=3)

def test_calldata_10_8():
    impl_test(10, depth=8)

def test_calldata_100():
    impl_test(100, depth=3)
