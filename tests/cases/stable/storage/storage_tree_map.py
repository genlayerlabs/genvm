# { "Depends": "py-genlayer:test" }

from genlayer import *
from genlayer.py.storage._internal.generate import generate_storage


@generate_storage
class UserStorage:
	m: TreeMap[str, u32]


tst = UserStorage()

tst.m['1'] = u32(12)
tst.m['2'] = u32(13)
del tst.m['1']
print('1' in tst.m, tst.m['2'])

exit(0)
