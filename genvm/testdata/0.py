# { "lang": "python" }

import genlayer.sdk as gsdk

def init():
    print(gsdk.message)
    print('init!')

gsdk.run(__import__(__name__))
