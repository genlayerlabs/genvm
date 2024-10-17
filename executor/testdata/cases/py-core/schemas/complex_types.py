# { "Depends": "genlayer-py-std:test" }
import genlayer.std as std
import typing


class MyTDict(typing.TypedDict):
	a: int
	b: str


@std.contract
class Contract:
	def __init__(self):
		pass

	@std.public
	def opt(
		self, a1: list | None, a2: typing.Union[str, bytes], a3: typing.Optional[str]
	):
		pass

	@std.public
	def lst(
		self, a1: list[str], a2: typing.Sequence[str], a3: typing.MutableSequence[int]
	):
		pass

	@std.public
	def dict(
		self,
		a1: dict[str, int],
		a2: typing.Mapping[str, str],
		a3: typing.MutableMapping[str, int | str | None],
	):
		pass
