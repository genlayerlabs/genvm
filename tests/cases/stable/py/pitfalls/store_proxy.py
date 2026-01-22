# { "Depends": "py-genlayer:test" }

from genlayer import *

from dataclasses import dataclass
import datetime


@allow_storage
@dataclass
class User:
	name: str
	birthday: datetime.datetime


class Contract(gl.Contract):
	users: DynArray[User]

	def __init__(self):
		self.users.append(User('Ada', datetime.datetime.now()))
		user = self.users[-1]
		self.users[-1] = User('Definitely not Ada', datetime.datetime.now())
		print(user.name)
		assert user.name == 'Definitely not Ada'  # this is true!
