# { "Depends": "py-genlayer:test" }

from genlayer import *
from genlayer.py.storage._internal.generate import storage

import numpy as np


@storage
class Test:
	foo: np.float32

	def abc(self):
		return self.foo


tst = Test()
tst.foo = np.float32(0.5)

assert tst.foo == 0.5, f'{tst.foo}'
assert type(tst.foo) is np.float32, f'{type(tst.foo)}'
print(tst.foo)

exit(0)
