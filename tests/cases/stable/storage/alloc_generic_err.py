# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *
from genlayer.storage import allow_storage
from dataclasses import dataclass
from genlayer.storage._internal.generate import generate_storage


@allow_storage
@dataclass
class Test[T]:
	foo: T


@generate_storage
class Bar:
	t: Test[str]


try:
	tst = Test('123')
	print(tst)
except Exception as e:
	print(e)
	for n in e.__notes__:
		print(n)

exit(0)
