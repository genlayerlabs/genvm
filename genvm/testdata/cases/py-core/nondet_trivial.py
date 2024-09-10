# { "depends": ["genvm-rustpython:test"] }
import genlayer.sdk as gsdk

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    def run(self):
        print('wow, nondet')
        return 'web page?'

@gsdk.public
def init():
    eval("print('init from eval!')")
    return gsdk.run_nondet({}, NonDetInit())
