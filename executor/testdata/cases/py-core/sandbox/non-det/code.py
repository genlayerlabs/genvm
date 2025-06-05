# { "Depends": "py-genlayer:test" }
from genlayer import *

import json


class Contract(gl.Contract):
	@gl.public.write
	def main(self, ev: str):
		def run_ndet():
			try:
				glb = globals()
				print(f'{gl.advanced.sandbox(lambda: eval(ev, glb)).get()}')
			except Rollback as rb:
				print(f'rollback {rb.msg}')
			except Exception as e:
				print(f'err {e.args}')
			print(json.loads.__name__)

		gl.eq_principle.strict_eq(run_ndet)
