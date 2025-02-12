# {
#   "Seq": [
#     { "Depends": "py-lib-genlayermodelwrappers:test" },
#     { "Depends": "py-genlayer:test" }
#   ]
# }

from genlayer import *
import genlayermodelwrappers


class Contract(gl.Contract):
	@gl.public.write
	def main(self, det: bool):
		embeddings_generator = genlayermodelwrappers.SentenceTransformer('all-MiniLM-L6-v2')

		def nd_block():
			real = embeddings_generator('what is genlayer?')
			print(real.sum())

		if det:
			nd_block()
		else:
			gl.eq_principle_strict_eq(nd_block)
