# { "depends": ["genvm-rustpython:test"] }

import genlayer.sdk as gsdk
from genlayer.py.types import *

@gsdk.storage
class UserStorage:
    m: gsdk.TreeMap[str, u32]

stor: UserStorage = UserStorage.view_at_root(gsdk.STORAGE_MAN)

@gsdk.public
def first():
    stor.__init__()
    print('first')
    stor.m['1'] = u32(12)
    stor.m['abc'] = u32(30)

@gsdk.public
def second():
    print('second')
    print(list(stor.m.items()))
