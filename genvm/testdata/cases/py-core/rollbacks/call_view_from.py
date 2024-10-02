# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.public
async def main(addr: gsdk.Address):
    print('contract from.main')
    try:
        res = await gsdk.Contract(addr).foo(1, 2)
    except gsdk.Rollback as r:
        print('handled', r.msg)
    else:
        print(res)
        exit(1)
