# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.vm import register_runner


class Contract(gl.contract.Contract):
	def __init__(self):
		# a custom runner whose module prints a marker when it is loaded
		code = b'# { "Depends": "py-genlayer:test" }\nprint("custom runner ran")\n'
		rid = register_runner(code)
		# run a sandbox that loads the custom runner instead of this contract
		res = gl.vm.spawn_sandbox(lambda: 42, runner=rid)
		print('sandbox ->', res)
