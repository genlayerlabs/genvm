# { "depends": ["genvm-rustpython:test"] }

import genlayer.sdk as gsdk
from genlayer.py.types import *

__gsdk_self_run__ = True

class Test:
    foo: i64
    bar: i64
    st: str

    def abc(self):
        return self.foo

@gsdk.storage
class Composite:
    a: Test
    b: Test

tst = Composite()

stor_man = tst._storage_slot.manager

tst.a.foo = 65535
tst.a.bar = 2 ** 32
tst.a.st = "123"
tst.b.foo = 13
tst.b.st = "321"

stor_man.debug()
print(tst.a.st, tst.b.st)

tst.a = tst.b

stor_man.debug()
print(tst.a.st, tst.b.st)
