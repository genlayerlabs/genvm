# {
#   "Seq": [
#     { "Depends": "py-lib-genlayer-embeddings:test" },
#     { "Depends": "py-genlayer:test" }
#   ]
# }


import numpy as np
import typing
from genlayer import *
import genlayer_embeddings as gle


class Contract(gl.Contract):
	x: gle.VecDB[np.float32, typing.Literal[5], str]

	@gl.public.write
	def main(self):
		self.x.insert(np.array([1, 2, 3, 4, 5], dtype=np.float32), '123')
		print(list(self.x.knn(np.ones(5, dtype=np.float32), 1)))
