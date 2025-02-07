# { "Depends": "py-genlayer:test" }
from genlayer import *

import json


class Contract(gl.Contract):
	@gl.public.write
	def main(self, ev: str):
		try:
			glb = globals()
			print(f'{gl.advanced.sandbox(lambda: eval(ev, glb)).get()}')
		except Rollback as rb:
			print(f'rollback {rb.msg}')
		except gl.advanced.ContractError as e:
			print(f'err {e.args}')
		print(json.loads.__name__)
