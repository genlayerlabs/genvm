# { "lang": "python" }
import genlayer.sdk as gsdk

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    def run(self):
        print('wow, nondet')
        return 'web page?'

@gsdk.public
def init():
    print(gsdk.message)
    eval("print('init from eval!')")
    return gsdk.run_nondet({}, NonDetInit())

gsdk.run(__import__(__name__))
