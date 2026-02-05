# { "Depends": "py-genlayer:test" }

from dataclasses import dataclass
import genlayer as gl
from genlayer.types import *
from genlayer.storage import allow_storage


@allow_storage
@dataclass
class Foo:
	x: gl.DynArray[str]


class Main(gl.contract.Contract):
	f: gl.DynArray[Foo]

	@gl.public.write
	def main(self):
		self.f.append(Foo(['123']))
		return [i for i in self.f]
