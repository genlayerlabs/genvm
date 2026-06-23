# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.vm import register_runner, map_file


class Contract(gl.contract.Contract):
	def __init__(self):
		# deterministic context registers a custom runner
		code = b'# { "Depends": "py-genlayer:test" }\nmarker\n'
		rid = register_runner(code)

		def leader():
			# burn a long loop first, then try to use the det-registered custom
			# runner from inside nondet. Custom runners should be scoped to the
			# execution that registered them, so this map MUST fail — but with a
			# shared registry it may leak.
			for _ in range(10**6):
				pass
			try:
				map_file(rid, 'file', '/mapped.txt')
				return 'nondet mapped det-registered runner (LEAK)'
			except Exception as e:
				return f'nondet map failed: {type(e).__name__}'

		print(gl.vm.run_nondet(leader, lambda r: True))
