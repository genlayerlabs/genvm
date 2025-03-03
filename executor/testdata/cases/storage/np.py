# { "Depends": "py-genlayer:test" }

import numpy as np

from genlayer import *

from genlayer.py.storage._internal.generate import storage


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
