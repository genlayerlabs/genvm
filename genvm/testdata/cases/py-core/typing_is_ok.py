# { "depends": ["genlayer-py-std:test"] }

#from __future__ import annotations

import typing
import types


__gsdk_self_run__ = True

class Foo[X]:
    a: X
    def __class_getitem__(self, k):
        return typing._GenericAlias(Foo, (k,))

def tst(x):
    print(f"=== {x.__name__}")
    print(type(x))
    print(x)
    if isinstance(x, types.GenericAlias):
        print(f'origin={x.__origin__}')
        print(f'args={x.__args__}')
    elif isinstance(x, typing._GenericAlias):
        print(f'origin={x.__origin__}')
        print(f'args={x.__args__}')
    if not isinstance(x, types.GenericAlias):
        for k, v in typing.get_type_hints(x).items():
            if isinstance(v, typing.TypeVar):
                v = v.__name__
            print(f"\t{k}: {v}")

tst(Foo)

tst(list[str])

class Test:
    foo: Foo[str]

tst(Test)
