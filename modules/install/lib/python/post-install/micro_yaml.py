# small zero-dependency YAML parser for a limited subset of YAML
# should be sufficient for our manifest files

from dataclasses import dataclass
import re
import typing
import enum
from copy import copy


class _Tok(enum.Enum):
	INDENT = 'INDENT'
	DEDENT = 'DEDENT'
	DASH = 'DASH'


@dataclass
class _MappingKey:
	key: str


@dataclass
class _Value:
	data: typing.Any


@dataclass
class _SpecialParser:
	regexp: re.Pattern
	constructor: typing.Callable[[str], _Value]


_special_dict = {
	'true': True,
	'false': False,
	'null': None,
	'[]': [],
	'{}': {},
}

_specials = [
	_SpecialParser(
		re.compile(r'^(true|false|null|\[\]|\{\})\s*(#|$)'),
		lambda s: copy(_special_dict[s]),
	),
	_SpecialParser(
		re.compile(r'^(-?\d+\.\d*([eE][+-]?\d+)?$|^-?\d*\.\d+([eE][+-]?\d+)?)\s*(#|$)'),
		lambda s: _Value(float(s)),
	),
	_SpecialParser(re.compile(r'^(-?\d+)\s*(#|$)'), lambda s: _Value(int(s))),
]


def _fetch_id(s: str) -> tuple[str, str]:
	if s.startswith('"'):
		end_idx = 1
		while end_idx < len(s):
			if s[end_idx] == '"' and s[end_idx - 1] != '\\':
				break
			end_idx += 1
		return s[1:end_idx], s[end_idx + 1 :].lstrip()
	end_idx = 0
	while end_idx < len(s) and not s[end_idx].isspace() and s[end_idx] != ':':
		end_idx += 1
	return s[:end_idx], s[end_idx:].lstrip()


def _is_comment_or_empty(s: str) -> bool:
	return s == '' or s.startswith('#')


def tokenize(
	lines: list[str],
) -> typing.Generator[_MappingKey | _Tok | _Value, None, None]:
	line_idx = 0
	indent_stack = [0]
	while line_idx < len(lines):
		line = lines[line_idx]
		rest = line.lstrip()
		indent = len(line) - len(rest)
		if _is_comment_or_empty(rest):
			line_idx += 1
			continue
		while indent_stack[-1] > indent:
			indent_stack.pop()
			yield _Tok.DEDENT
		if indent_stack[-1] < indent:
			indent_stack.append(indent)
			yield _Tok.INDENT

		if rest.startswith('-'):
			yield _Tok.DASH
			rest = rest[1:].lstrip()
		else:
			ident, rest = _fetch_id(rest)
			if len(ident) == 0:
				raise ValueError(f'Invalid YAML line: {line_idx}')
			if not rest.startswith(':'):
				raise ValueError(f'Expected ":" after key in line: {line_idx}')
			rest = rest[1:].lstrip()
			yield _MappingKey(ident)

		if _is_comment_or_empty(rest):
			line_idx += 1
			continue

		# here we need to parse value

		### |
		if rest.startswith('|'):
			rest = rest[1:].lstrip()
			if not _is_comment_or_empty(rest):
				raise ValueError(f'Unexpected content after block scalar in line: {line_idx}')
			line_idx += 1
			if line_idx >= len(lines):
				yield _Value('')
				continue
			left_indent = 0
			while left_indent < len(lines[line_idx]) and lines[line_idx][left_indent] == ' ':
				left_indent += 1
			expected_indent = ' ' * left_indent
			value_lines = []
			while line_idx < len(lines):
				next_line = lines[line_idx]
				if next_line.startswith(expected_indent):
					value_lines.append(next_line[left_indent:])
					line_idx += 1
				else:
					if _is_comment_or_empty(next_line.lstrip()):
						line_idx += 1
						value_lines.append('')
						continue
					else:
						break
			yield _Value('\n'.join(value_lines))
			continue

		### single line values ###
		if rest.startswith('"'):
			data, rest = _fetch_id(rest)
			yield _Value(data)
			if not _is_comment_or_empty(rest):
				raise ValueError(f'Unexpected value in line: {line_idx}')
			line_idx += 1
			continue
		found = False
		for sp in _specials:
			match = sp.regexp.match(rest)
			if match is not None:
				# special value
				yield sp.constructor(match.group(0))
				rest = rest[len(match.group(0)) :].lstrip()
				found = True
				break
		if not found:
			hash_idx = rest.find('#')
			if hash_idx == -1:
				hash_idx = len(rest)
			yield _Value(rest[:hash_idx])
			rest = rest[hash_idx:].lstrip()
		if not _is_comment_or_empty(rest):
			raise ValueError(f'Unexpected value in line: {line_idx}')
		line_idx += 1

	while len(indent_stack) > 1:
		indent_stack.pop()
		yield _Tok.DEDENT


from collections import deque


class _Poller:
	def __init__(self, tokens: typing.Generator[_MappingKey | _Tok | _Value, None, None]):
		self._tokens = tokens
		self._head = None
		self.done = False

	def peek(self) -> _MappingKey | _Tok | _Value | None:
		if self._head is not None:
			return self._head
		if self.done:
			return None

		try:
			self._head = next(self._tokens)
			return self._head
		except StopIteration:
			self.done = True
			return None

	def fetch(self) -> _MappingKey | _Tok | _Value | None:
		r = self._fetch()
		return r

	def _fetch(self) -> _MappingKey | _Tok | _Value | None:
		if self._head is not None:
			res = self._head
			self._head = None
			return res
		if self.done:
			return None
		try:
			return next(self._tokens)
		except StopIteration:
			self.done = True
			return None


def _parse(poller: _Poller) -> typing.Any:
	data = poller.peek()
	if data is None:
		raise ValueError('Unexpected end of input')
	if data == _Tok.INDENT:
		poller.fetch()  # consume INDENT
		ret = _parse(poller)
		dedent = poller.fetch()
		if dedent != _Tok.DEDENT:
			raise ValueError('Expected DEDENT token')
		return ret
	if isinstance(data, _Value):
		poller.fetch()  # consume VALUE
		return data.data
	if data == _Tok.DASH:
		ret = []
		while True:
			next_tok = poller.peek()
			if next_tok != _Tok.DASH:
				break
			poller.fetch()  # consume DASH
			val = _parse(poller)
			ret.append(val)
		return ret
	elif isinstance(data, _MappingKey):
		ret = {}
		while True:
			next_tok = poller.peek()
			if not isinstance(next_tok, _MappingKey):
				break
			key = next_tok.key
			poller.fetch()  # consume MAPPING KEY
			val = _parse(poller)
			ret[key] = val
		return ret
	else:
		raise ValueError(f'Unexpected token: {data}')


def loads(s: str) -> typing.Any:
	lines = s.splitlines()
	poller = _Poller(tokenize(lines))
	data = _parse(poller)
	if poller.peek() is not None:
		raise ValueError('Unexpected data after end of document')
	return data
