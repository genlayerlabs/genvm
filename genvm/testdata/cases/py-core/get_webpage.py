# { "depends": ["genvm-rustpython:test"] }
import genlayer.sdk as gsdk
import json

class NonDetInit(gsdk.Runner):
    def __init__(self, mode):
        self.mode = mode
    def run(self):
        return gsdk.wasi.get_webpage(json.dumps({"mode": self.mode}), "http://127.0.0.1:4242/hello.html")

@gsdk.public
def main(mode: str):
    print(gsdk.run_nondet({"mode": "refl"}, NonDetInit(mode)))
