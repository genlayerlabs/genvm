# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.vm import register_runner

import json


class Contract(gl.contract.Contract):
	def __init__(self):
		runner = {
			'Seq': [
				{
					'With': {
						'action': {'MapFile': {'file': 'file', 'to': '/contract.py'}},
						'runner': 'contract',
					}
				},
				{'SetArgs': ['py', '-u', '-B', '/contract.py']},
				{'Depends': 'py-lib-cloudpickle:test'},
				{'Depends': 'py-lib-genlayer-std:test'},
				{'Depends': 'cpython:test'},
			]
		}
		runner_dumped = json.dumps(runner)
		code = (
			f'# {runner_dumped}\n'
			'print("custom runner ran")\n'
			"exec(open('/py/libs/_genlayer_bootloader.py').read())\n"
		)
		rid = register_runner(code.encode('utf-8'))
		res = gl.vm.spawn_sandbox(lambda: 42, runner=rid)
		print('sandbox ->', res)
