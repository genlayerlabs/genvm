# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    def run(self):
        gsdk.rollback_immediate("nah, I won't execute")

@gsdk.contract
class Contract:
    @gsdk.public
    async def main(self):
        try:
            res = await gsdk.run_nondet({"mode": "refl"}, NonDetInit())
        except gsdk.Rollback as r:
            print('handled', r.msg)
        else:
            print(res)
            exit(1)
