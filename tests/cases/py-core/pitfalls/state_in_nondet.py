# { "Depends": "py-genlayer:test" }

from genlayer import *

from dataclasses import dataclass
import datetime


@allow_storage
@dataclass
class User:
	name: str
	birthday: datetime.datetime


@allow_storage
@dataclass
class Gen[T]:
	name: T
	birthday: datetime.datetime


class Contract(gl.Contract):
	user: User
	gen_user: Gen[bytes]

	@gl.public.write
	def plain(self):
		user = User('Ada', datetime.datetime.now())
		self.user = user

		read_user = self.user

		copied_out = gl.storage.copy_to_memory(read_user)

		def nd():
			print('inmem: ok', user)
			try:
				print('storage: not ok', str(read_user))
			except Exception as e:
				print('storage: not ok', e)
			print('copied out: ok', copied_out)

		gl.eq_principle.strict_eq(nd)

	@gl.public.write
	def generic(self):
		user = gl.storage.inmem_allocate(Gen[bytes], b'Ada', datetime.datetime.now())
		self.gen_user = user

		read_user = self.gen_user

		copied_out = gl.storage.copy_to_memory(read_user)

		def nd():
			print('inmem: ok', user)
			try:
				print('storage: not ok', str(read_user))
			except Exception as e:
				print('storage: not ok', e)
			print('copied out: ok', copied_out)

		gl.eq_principle.strict_eq(nd)
