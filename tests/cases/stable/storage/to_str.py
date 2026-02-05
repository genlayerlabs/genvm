# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *
from genlayer.storage import allow_storage

from dataclasses import dataclass
import datetime


@allow_storage
@dataclass
class User:
	name: str
	birthday: datetime.datetime


class LlmErc20(gl.contract.Contract):
	x: User

	def __init__(self) -> None:
		print(str(self.x))
