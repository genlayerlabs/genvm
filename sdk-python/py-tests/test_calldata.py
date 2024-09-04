import genlayer.calldata as calldata
from genlayer.types import Address

import random

def byte_range(first, last):
    return list(range(first, last+1))

first_values = byte_range(0x00, 0x7F) + byte_range(0xC2, 0xF4)
trailing_values = byte_range(0x80, 0xBF)


def random_utf8_codepoint():
    first = random.choice(first_values)
    if first <= 0x7F:
        return bytes([first])
    elif first <= 0xDF:
        return bytes([first, random.choice(trailing_values)])
    elif first == 0xE0:
        return bytes([first, random.choice(byte_range(0xA0, 0xBF)), random.choice(trailing_values)])
    elif first == 0xED:
        return bytes([first, random.choice(byte_range(0x80, 0x9F)), random.choice(trailing_values)])
    elif first <= 0xEF:
        return bytes([first, random.choice(trailing_values), random.choice(trailing_values)])
    elif first == 0xF0:
        return bytes([first, random.choice(byte_range(0x90, 0xBF)), random.choice(trailing_values), random.choice(trailing_values)])
    elif first <= 0xF3:
        return bytes([first, random.choice(trailing_values), random.choice(trailing_values), random.choice(trailing_values)])
    elif first == 0xF4:
        return bytes([first, random.choice(byte_range(0x80, 0x8F)), random.choice(trailing_values), random.choice(trailing_values)])

def random_str(size):
    mem = bytearray()
    for x in range(size):
        mem.extend(random_utf8_codepoint())
    return str(mem, encoding='utf-8')

def make_rnd(size_limit, rem_depth):
    if rem_depth <= 0:
        r = random.randint(0, 4)
        if r == 0:
            return random.randint(-10**8, 10**8)
        if r == 1:
            return Address(random.randbytes(32))
        if r == 2:
            return random.randbytes(random.randint(0, size_limit))
        if r == 3:
            return random_str(size_limit)
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
        ret[random_str(20)] = make_rnd(size_limit, rem_depth - 1)
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
