# {
#   "Seq": [
#     { "Depends": "py-lib-genlayer-embeddings:test" },
#     { "Depends": "py-genlayer:test" }
#   ]
# }


import numpy as np
import typing
import genlayer as gl
from genlayer.types import *
import genlayer_embeddings as gle


class Contract(gl.contract.Contract):
	x: gle.VecDB[np.float32, typing.Literal[5], str, gle.EuclideanDistance]

	def __init__(self):
		self.x.insert(np.array([1, 2, 3, 4, 5], dtype=np.float32), '123')
		print(list(self.x.knn(np.ones(5, dtype=np.float32), 1)))
