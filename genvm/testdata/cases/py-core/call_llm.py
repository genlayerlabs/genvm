# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk
import json

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    async def run(self):
        return await gsdk.call_llm({}, "print 'yes' (without quotes) and nothing else")

@gsdk.contract
class Contract:
    @gsdk.public
    async def main(self):
        print(await gsdk.run_nondet({"mode": "refl"}, NonDetInit()))
