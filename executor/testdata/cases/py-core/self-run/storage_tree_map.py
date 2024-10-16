# { "depends": ["genlayer-py-std:test"] }

import genlayer.std as gl
from genlayer.py.types import *
from genlayer.py.storage import storage, TreeMap

__gsdk_self_run__ = True


@storage
class UserStorage:
	m: TreeMap[str, u32]


tst = UserStorage()

tst.m['1'] = 12
tst.m['2'] = 13
del tst.m['1']
print('1' in tst.m, tst.m['2'])
