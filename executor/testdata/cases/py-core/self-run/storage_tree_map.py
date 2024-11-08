# { "Depends": "py-genlayer:test" }

from genlayer import *
from genlayer.py.storage.generate import storage


@storage
class UserStorage:
	m: TreeMap[str, u32]


tst = UserStorage()

tst.m['1'] = 12
tst.m['2'] = 13
del tst.m['1']
print('1' in tst.m, tst.m['2'])

exit(0)
