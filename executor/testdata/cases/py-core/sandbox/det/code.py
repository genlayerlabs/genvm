# { "Depends": "py-genlayer:test" }
from genlayer import *

import json


@gl.contract
class Contract:
	@gl.public.write
	def main(self, ev: str):
		try:
			glb = globals()
			print(f'{gl.sandbox(lambda: eval(ev, glb))}')
		except Rollback as rb:
			print(f'rollback {rb.msg}')
		except Exception as e:
			print(f'err {e.args}')
		print(json.loads.__name__)
