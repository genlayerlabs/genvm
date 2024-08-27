# { "lang": "python" }
import genlayer.sdk as gsdk

@gsdk.public
def main(addr: str):
    print('contract from.main')
    print(gsdk.Contract(gsdk.Address(addr)).foo(1, 2))

gsdk.run(__import__(__name__))
