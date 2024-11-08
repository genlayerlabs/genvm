# { "Depends": "py-genlayer:test" }
from genlayer import *
import typing


class MyTDict(typing.TypedDict):
	a: int
	b: str


@gl.contract
class Contract:
	def __init__(self):
		pass

	@gl.public.write
	def opt(
		self, a1: list | None, a2: typing.Union[str, bytes], a3: typing.Optional[str]
	):
		pass

	@gl.public.write
	def lst(
		self, a1: list[str], a2: typing.Sequence[str], a3: typing.MutableSequence[int]
	):
		pass

	@gl.public.write
	def dict(
		self,
		a1: dict[str, int],
		a2: typing.Mapping[str, str],
		a3: typing.MutableMapping[str, int | str | None],
	):
		pass
