# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.storage.root import Root


class Contract(gl.contract.Contract):
	def __init__(self):
		# persist a major that does not match the node's, so every later read of
		# this contract's runner (top-level call, CallContract, map_file) is
		# rejected before it runs
		Root.get().major = 5
		print('deployed target with major 5')

	@gl.public.view
	def foo(self) -> int:
		print('should not be reached')
		return 1
