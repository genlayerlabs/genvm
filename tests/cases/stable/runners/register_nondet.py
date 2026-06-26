# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.vm import register_runner


class Contract(gl.contract.Contract):
	def __init__(self):
		def leader():
			# register_runner is deterministic-only; calling it from inside a
			# nondet block must be refused even though `u` (register) is granted,
			# i.e. the rejection comes from the is_deterministic gate, not perms.
			try:
				register_runner(b'# { "Depends": "py-genlayer:test" }\nx\n')
				return 'registered (LEAK)'
			except Exception as e:
				return f'forbidden: {type(e).__name__}'

		print(gl.vm.run_nondet(leader, lambda r: True))
