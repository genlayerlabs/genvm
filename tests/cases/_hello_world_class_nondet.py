# v0.1.10
# {
#   "Seq": [
#     { "AddEnv": {"name": "GENLAYER_ENABLE_PROFILER", "val": "false"} },
#     { "Depends": "py-genlayer:test" }
#   ]
# }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def foo(self):
		gl.vm.run_nondet_unsafe(lambda: None, lambda x: True)
		print('hello world')
