# {
#   "Seq": [
#     { "Depends": "py-lib-genlayer-embeddings:test" },
#     { "Depends": "py-genlayer:test" }
#   ]
# }

import genlayer as gl
from genlayer.types import *
import genlayer_embeddings as gle


class Contract(gl.contract.Contract):
	@gl.public.write
	def main(self, det: bool):
		def nd_block():
			embeddings_generator = gle.SentenceTransformer('all-MiniLM-L6-v2')
			real = embeddings_generator('what is genlayer?')
			print(real.sum())

		if det:
			nd_block()
		else:
			gl.eq_principle.strict_eq(nd_block)
