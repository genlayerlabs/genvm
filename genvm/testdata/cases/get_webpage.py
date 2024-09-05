# { "lang": "python" }
import genlayer.sdk as gsdk

class NonDetInit(gsdk.Runner):
    def __init__(self):
        pass
    def run(self):
        contents = gsdk.wasi.get_webpage("{}", "http://127.0.0.1:4242/hello.html")
        print(contents)

@gsdk.public
def main():
    gsdk.run_nondet({}, NonDetInit())
