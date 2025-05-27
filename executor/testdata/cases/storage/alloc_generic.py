# { "Depends": "py-genlayer:test" }

from genlayer import *
from dataclasses import dataclass


@allow_storage
@dataclass
class Test[T]:
	foo: T


tst = gl.storage.inmem_allocate(Test[str], '123')
print(tst)

exit(0)
