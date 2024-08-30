# { "lang": "python" }

import genlayer.sdk as gsdk
from genlayer.types import *
from genlayer.storage import storage, TreeMap

__gsdk_self_run__ = True


@storage
class UserStorage:
    m: TreeMap[str, u32]

tst = UserStorage.init_at()

tst['1'] = 12
tst['2'] = 13
del tst['1']
print('1' in tst, tst['2'])
