# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.types import *


class Contract(gl.contract.Contract):
	def __init__(self):
		# self-CallContract into our own view method during deploy; arguably a
		# footgun that should be forbidden
		try:
			print('self view ->', gl.contract.get_at(self.address).view().read30())
		except Exception as e:
			print('self view failed:', type(e).__name__)

	@gl.public.view
	def read30(self) -> int:
		return 30
