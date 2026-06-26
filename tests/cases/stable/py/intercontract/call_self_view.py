# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		# self-CallContract into our own view method during deploy: our code is
		# not committed on-chain yet, so it is not visible as an `:a` runner and
		# this fails with `invalid_contract` (aborting the deploy)
		print('self view ->', gl.contract.get_at(self.address).view().read30())

	@gl.public.view
	def read30(self) -> int:
		return 30
