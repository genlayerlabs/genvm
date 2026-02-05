# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		def run():
			return (
				gl.nondet.exec_prompt(
					"respond with two letters 'OK' (without quotes) and nothing else, no repetition"
				)
				.strip()
				.lower()
			)

		print(gl.eq_principle.prompt_comparative(run, 'result must be exactly the same'))
