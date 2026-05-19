# v0.1.10
# {
#   "Seq": [
#     { "AddEnv": {"name": "GENLAYER_ENABLE_PROFILER", "val": "false"} },
#     { "Depends": "py-genlayer:test" }
#   ]
# }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		gl.vm.run_nondet(lambda: None, lambda x: True)
		print('hello world')
