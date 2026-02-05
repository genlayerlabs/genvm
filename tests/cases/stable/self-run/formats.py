# { "Depends": "py-genlayer:test" }

from genlayer.storage._internal.generate import generate_storage
import genlayer as gl
from genlayer.types import Address


@generate_storage
class Test:
	arr: gl.DynArray[str]
	map: gl.TreeMap[str, None]


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
