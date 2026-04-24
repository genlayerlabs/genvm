# { "Depends": "py-genlayer:test" }
import sys

import genlayer as gl
from genlayer.types import *

import genlayer._internal.on_chain.gl_call as gl_call
from genlayer import calldata
import typing


def _decode_nondet(buf):
	ret = typing.cast(dict, calldata.decode(buf))
	if err := ret.get('error'):
		print('ERROR')
		exit(1)
	return ret['ok']


def exec_prompt(
	prompt: str,
	response_format: str,
):
	return gl_call.gl_call_generic(
		{
			'ExecPrompt': {
				'prompt': prompt,
				'response_format': response_format,
				'images': [],
			}
		},
		_decode_nondet,
	).get()


class Contract(gl.contract.Contract):
	def __init__(self):
		def run():
			v = exec_prompt(
				'Respond with a json object with a single key "random" and value between 0 and 1, like 0.5',
				response_format='json',
			)
			print(v, file=sys.stderr)
			print(type(v).__name__)
			r = v['random']
			print(type(r).__name__)

			r1 = float(r)
			print(r1 >= 0 and r1 <= 1)

		gl.eq_principle.strict_eq(run)
