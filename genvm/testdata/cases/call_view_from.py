# { "depends": ["genvm-rustpython:test"] }
import genlayer.sdk as gsdk

@gsdk.public
def main(addr: gsdk.Address):
    print('contract from.main')
    print(gsdk.Contract(addr).foo(1, 2))
