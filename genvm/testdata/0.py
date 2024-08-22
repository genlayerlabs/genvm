# { "lang": "python" }

import genlayer.sdk as gsdk

#def never_called():
#    x = 0.5

def init():
    print(gsdk.message)
    eval("print('init from eval!')")

gsdk.run(__import__(__name__))
