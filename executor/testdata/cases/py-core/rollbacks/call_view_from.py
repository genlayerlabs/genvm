# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.contract
class Contract:
    @gsdk.public
    async def main(self, addr: gsdk.Address):
        print('contract from.main')
        try:
            res = await gsdk.OtherContract(addr).foo(1, 2)
        except gsdk.Rollback as r:
            print('handled', r.msg)
        else:
            print(res)
            exit(1)
