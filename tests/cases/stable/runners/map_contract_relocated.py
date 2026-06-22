# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.storage.root import Root
from genlayer.vm import public_abi

# the relocated contract: a script that maps its OWN runner ("contract") and
# checks it received real code. If `map_file("contract")` re-read the (wiped)
# default code slot it would get 0 bytes here.
C2 = b"""\
# { "Depends": "py-genlayer:test" }
import genlayer as gl
from genlayer.vm import map_file
class Contract(gl.contract.Contract):
	@gl.public.write
	def foo(self):
		map_file("contract", "file", "/x")
		d = open("/x", "rb").read()
		print("ok=" + str(len(d) > 0 and d[:3] == b"# {"))
"""


class Contract(gl.contract.Contract):
	def __init__(self):
		# move the code to slot 7, point code_slot there, then wipe the default
		# code slot (offset 1)
		sid = bytes([7]) + bytes(31)
		s = Root.MANAGER.get_store_slot(sid)
		new_code_vla = s.cast(gl.storage.VLA[gl.u8], 0)
		new_code_vla.assign(C2)
		Root.get().code_slot = s.as_int()
		Root.get().code.get().truncate()
		print('deployed')

	@gl.public.write
	def foo(self) -> None:
		print('C1.foo should not run')
