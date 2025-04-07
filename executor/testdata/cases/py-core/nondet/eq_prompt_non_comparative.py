# { "Depends": "py-genlayer:test" }
from genlayer import *
import sys


class Contract(gl.Contract):
	@gl.public.write
	def main(self):
		def get_input():
			return "As pets, rats are affectionate, playful, and form strong bonds with their human companions. They're curious, enjoy interactive toys, and can learn tricks much like small dogs. Their adaptability, intelligence, and charming personalities make them truly cool animals that deserve much more appreciation than they currently get."

		print(
			gl.eq_principle_prompt_non_comparative(
				get_input,
				task='Produce a text summary',
				criteria='It must be at least two times less than the input (in either words or characters). This means that for 40 words valid summaries could be 10, 19, ... words long',
			),
			file=sys.stderr,
		)
