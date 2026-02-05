# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *
from genlayer.storage._internal.generate import generate_storage


@generate_storage
class Test:
	foo: float

	def abc(self):
		return self.foo


tst = Test()
tst.foo = 0.5

assert tst.foo == 0.5
assert type(tst.foo) is float
print(tst.foo)

exit(0)
