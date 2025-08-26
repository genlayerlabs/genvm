# { "Depends": "py-genlayer:test" }

from dataclasses import dataclass
from genlayer import *


@allow_storage
@dataclass
class Foo:
	x: DynArray[str]


class Main(gl.Contract):
	f: DynArray[Foo]

	@gl.public.write
	def main(self):
		self.f.append(Foo(['123']))
		return [i for i in self.f]
