# { "depends": ["genlayer-py-std:test"] }
import genlayer.sdk as gsdk

@gsdk.public
def pub():
    eval("print('init from pub!')")

@gsdk.public
def rback():
    gsdk.rollback_immediate("nah, I won't execute")

def priv():
    eval("print('init from priv!')")

@gsdk.public
def retn():
    return {"x": 10}

@gsdk.public
def retn_ser():
    return gsdk.AlreadySerializedResult(b"123")

@gsdk.public
def det_viol():
    import json
    gsdk.wasi.get_webpage(json.dumps({"mode": "text"}), "http://127.0.0.1:4242/hello.html")
