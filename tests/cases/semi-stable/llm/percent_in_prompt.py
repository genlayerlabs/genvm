# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def run():
			return '%0'

		print(gl.eq_principle.prompt_comparative(run, 'result must be exactly the same'))
