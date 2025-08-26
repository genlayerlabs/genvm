# { "Depends": "py-genlayer:test" }

from genlayer import *
from genlayer.py.storage._internal.generate import generate_storage


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
