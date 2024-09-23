# { "depends": ["genvm-rustpython:test"] }
import genlayer.sdk as gsdk

@gsdk.public
async def main(addr: gsdk.Address):
    print('contract from.main')
    print(await gsdk.Contract(addr).foo(1, 2))
