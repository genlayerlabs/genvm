# { "depends": ["genvm-rustpython:test"] }
import genlayer.sdk as gsdk
import json

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    def run(self):
        return gsdk.wasi.call_llm(json.dumps({}), "print yes and nothing else")

@gsdk.public
def main():
    print(gsdk.run_nondet({"mode": "refl"}, NonDetInit()))
