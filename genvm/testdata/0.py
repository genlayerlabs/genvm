# { "lang": "python" }
import genlayer.sdk as gsdk
print(1.0 * 3.0)
print('123')

def init():
    print(gsdk.message)
    eval("print('init from eval!')")
    return "ha-ha"

gsdk.run(__import__(__name__))
