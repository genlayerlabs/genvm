# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.storage.root import Root


class Contract(gl.contract.Contract):
	def __init__(self):
		# write a *different* valid contract to slot 7 and point code_slot at it;
		# the next invocation must load and run THAT code, not this class's `foo`.
		alt = (
			b'# { "Depends": "py-genlayer:test" }\nprint("hello from code_slot")\nexit(0)\n'
		)
		slot_id = bytes([7]) + bytes(31)
		s = Root.MANAGER.get_store_slot(slot_id)
		s.write(0, len(alt).to_bytes(4, 'little'))
		s.write(4, alt)
		Root.get().code_slot = int.from_bytes(slot_id, 'little')
		print('deployed')

	@gl.public.write
	def foo(self) -> None:
		# only reached if code_slot was IGNORED (default code loaded)
		print('original foo ran -- code_slot ignored!')
