# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    def run(self):
        print('wow, nondet')
        return 'web page?'

@gsdk.public
async def init():
    eval("print('init from eval!')")
    return await gsdk.run_nondet({"mode": "refl"}, NonDetInit())
