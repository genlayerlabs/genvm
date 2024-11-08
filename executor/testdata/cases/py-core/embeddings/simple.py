# {
#   "Seq": [
#     { "Depends": "py-lib-genlayermodelwrappers:KA27MQRPHZJQKSOOLOHWX2SI5X5C3MDO55H6XENBG72ZA3QKQKY43ZA6ZAHO47FSPFMAHGOOQT2P5RKJTCKPK72AYQU6A5RGN3E5DBA=" },
#     { "Depends": "py-genlayer:test" }
#   ]
# }

from genlayer import *
import genlayermodelwrappers


@gl.contract
class Contract:
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
