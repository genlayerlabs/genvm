# { "Depends": "py-genlayer:test" }
from genlayer import *

import json


class Contract(gl.Contract):
	@gl.public.write
	def main(self, ev: str):
		glb = globals()
		print(f'{gl.vm.spawn_sandbox(lambda: eval(ev, glb))}')
		print(json.loads.__name__)
