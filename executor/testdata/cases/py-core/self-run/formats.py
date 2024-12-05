# { "Depends": "py-genlayer:test" }

from genlayer.py.storage.generate import storage
from genlayer import *


@storage
class Test:
	arr: DynArray[str]
	map: TreeMap[str, None]


addr = Address(b'\xa2' * 20)
print(addr)
print(f'{addr}')
print(f'{addr!r}')
print(f'{addr!s}')
print(f'{addr:cd}')
print(f'{addr:b64}')
print(f'{addr:x}')
print(f'{addr:}')

t = Test()
t.arr.extend(['1', '2', '3'])
t.map.update({'1': None, '4': None})
print(f'{t.arr}')
print(f'{t.map}')
exit(0)
