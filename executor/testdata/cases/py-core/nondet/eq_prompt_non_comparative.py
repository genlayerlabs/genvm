# { "Depends": "py-genlayer:test" }
from genlayer import *
import sys


@gl.contract
class Contract:
	@gl.public.write
	def main(self):
		def get_input():
			return "As pets, rats are affectionate, playful, and form strong bonds with their human companions. They're curious, enjoy interactive toys, and can learn tricks much like small dogs. Their adaptability, intelligence, and charming personalities make them truly cool animals that deserve much more appreciation."

		print(
			gl.eq_principle_prompt_non_comparative(
				get_input,
				task='Produce a text summary',
				criteria='it must be at least two times less than the input (in words)',
			),
			file=sys.stderr,
		)
