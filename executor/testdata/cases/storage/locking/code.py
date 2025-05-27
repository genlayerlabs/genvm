# { "Depends": "py-genlayer:test" }

from genlayer import *

new_code = b"""\
# { "Depends": "py-genlayer:test" }
print("Patched!")
exit(30)
"""


class Contract(gl.Contract):
	def __init__(self, modifiers: list[Address], modify_in_ctor: bool):
		root = gl.storage.Root.get()
		root.upgraders.get().extend(modifiers)

		self.show_info()

		if modify_in_ctor:
			self.try_modify()

	def show_info(self):
		root = gl.storage.Root.get()

		print(list(root.locked_slots.get()))
		print(list(root.upgraders.get()))

	@gl.public.write
	def nop(self):
		pass

	@gl.public.write
	def try_modify(self):
		root = gl.storage.Root.get()

		self.show_info()

		code = root.code.get()
		try:
			code.truncate()  # <- we should encounter error here
		except BaseException as e:
			print(e)
		else:
			code.extend(new_code)
		print(len(code))
