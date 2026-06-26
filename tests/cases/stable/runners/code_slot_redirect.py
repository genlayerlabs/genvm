# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.storage.root import Root


class Contract(gl.contract.Contract):
	def __init__(self):
		# redirect the code slot to a different (empty) slot; the next invocation
		# must read its code from there instead of the default slot
		Root.get().code_slot = 12345
		print('deployed')

	@gl.public.write
	def foo(self) -> None:
		print('should not be reached')
