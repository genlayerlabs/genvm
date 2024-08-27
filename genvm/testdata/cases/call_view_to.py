# { "lang": "python" }
import genlayer.sdk as gsdk

@gsdk.public
def foo(a, b):
    print('contract to.foo')
    import json
    json.loads = 11 # evil!
    return a + b

gsdk.run(__import__(__name__))
