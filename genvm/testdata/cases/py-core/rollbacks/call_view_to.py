# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.public
def foo(a, b):
    print('contract to.foo')
    gsdk.rollback_immediate(f"nah, I won't execute {a + b}")
