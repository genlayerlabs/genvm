# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.storage.root import Root


class Contract(gl.contract.Contract):
	def __init__(self):
		# write a major that does not match the node's, so the *next* invocation
		# is rejected before it runs
		Root.get().major = 5
		print('deployed')

	@gl.public.write
	def foo(self) -> None:
		print('should not be reached')
