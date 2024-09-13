# { "depends": ["genvm-rustpython:test"] }
import genlayer.sdk as gsdk
import json

class NonDetInit(gsdk.Runner):
    def __init__(self, mode):
        self.mode = mode
        pass
    def run(self):
        contents = gsdk.wasi.get_webpage(json.dumps({"mode": self.mode}), "http://127.0.0.1:4242/hello.html")
        print(contents)

@gsdk.public
def main(mode: str):
    gsdk.run_nondet({"mode": "refl"}, NonDetInit(mode))
