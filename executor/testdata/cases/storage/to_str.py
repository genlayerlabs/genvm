# { "Depends": "py-genlayer:test" }

from genlayer import *

from dataclasses import dataclass
import datetime


@allow_storage
@dataclass
class User:
	name: str
	birthday: datetime.datetime


class LlmErc20(gl.Contract):
	x: User

	@gl.public.write
	def main(self) -> None:
		print(str(self.x))
