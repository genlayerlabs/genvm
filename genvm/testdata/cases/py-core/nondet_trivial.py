# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.contract
class Contract:
    @gsdk.public
    async def init(self):
        eval("print('init from eval!')")
        def run():
            print('wow, nondet')
            return 'web page?'
        return await gsdk.run_nondet({"mode": "refl"}, run)
