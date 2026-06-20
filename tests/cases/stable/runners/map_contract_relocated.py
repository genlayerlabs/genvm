# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.storage.root import Root
from genlayer.vm import public_abi

# the relocated contract: a script that maps its OWN runner ("contract") and
# checks it received real code. If `map_file("contract")` re-read the (wiped)
# default code slot it would get 0 bytes here.
C2 = (
	b'# { "Depends": "py-genlayer:test" }\n'
	b'import genlayer as gl\n'
	b'from genlayer.vm import map_file\n'
	b'class Contract(gl.contract.Contract):\n'
	b'\t@gl.public.write\n'
	b'\tdef foo(self):\n'
	b'\t\tmap_file("contract", "file", "/x")\n'
	b'\t\td = open("/x", "rb").read()\n'
	b'\t\tprint("ok=" + str(len(d) > 0 and d[:3] == b"# {"))\n'
)


class Contract(gl.contract.Contract):
	def __init__(self):
		# move the code to slot 7, point code_slot there, then wipe the default
		# code slot (offset 1)
		sid = bytes([7]) + bytes(31)
		s = Root.MANAGER.get_store_slot(sid)
		s.write(0, len(C2).to_bytes(4, 'little'))
		s.write(4, C2)
		Root.get().code_slot = int.from_bytes(sid, 'little')
		default = Root.get().slot().indirect(public_abi.root_offsets.CODE)
		default.write(0, (0).to_bytes(4, 'little'))
		print('deployed')

	@gl.public.write
	def foo(self) -> None:
		print('C1.foo should not run')
