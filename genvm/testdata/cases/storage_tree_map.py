# { "runner": ["genvm-rustpython:test"] }

import genlayer.sdk as gsdk
from genlayer.types import *

__gsdk_self_run__ = True


@gsdk.storage
class UserStorage:
    m: gsdk.TreeMap[str, u32]

tst = UserStorage()

tst.m['1'] = 12
tst.m['2'] = 13
del tst.m['1']
print('1' in tst.m, tst.m['2'])
