from pathlib import Path
import re
import typing


class Filter(typing.Protocol):
	def __call__(self, key: str, value: str | Path) -> bool: ...


class FilterAll:
	def __init__(self, *sub: Filter) -> None:
		self.sub = sub

	def __call__(self, key: str, value: str | Path) -> bool:
		return all(f(key, value) for f in self.sub)


class FilterAny:
	def __init__(self, *sub: Filter) -> None:
		self.sub = sub

	def __call__(self, key: str, value: str | Path) -> bool:
		return any(f(key, value) for f in self.sub)


class FilterKeyRe:
	def __init__(self, pattern: str) -> None:
		self.re = re.compile(pattern)

	def __call__(self, key: str, _value: str | Path) -> bool:
		return bool(self.re.search(key))


class FilterKey:
	def __init__(self, *keys: str) -> None:
		self.keys = keys

	def __call__(self, key: str, _value: str | Path) -> bool:
		return key in self.keys


DEFAULT_FILTER: FilterAny = FilterAny(
	FilterKeyRe(r'^(PATH|HOME|USER|SHELL|LOGNAME|PWD|TERM|LD_LIBRARY_PATH)$'),
	FilterKeyRe(r'^(PYTHON|VIRTUAL_ENV|CARGO|RUST)'),
)
