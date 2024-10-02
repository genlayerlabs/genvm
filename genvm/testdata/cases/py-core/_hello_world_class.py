# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.contract
class Contract:
    @gsdk.public
    def __init__(self):
        print("hello world")
