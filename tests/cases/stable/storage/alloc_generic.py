# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *
from genlayer.storage import allow_storage
from dataclasses import dataclass


@allow_storage
@dataclass
class Test[T]:
	foo: T


tst = gl.storage.inmem_allocate(Test[str], '123')
print(tst)

exit(0)
