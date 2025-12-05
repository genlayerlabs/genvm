import abc
import collections.abc
import enum
import io
import json
from pathlib import Path
import threading
import traceback


class Level(enum.Enum):
	TRACE = 0
	DEBUG = 10
	INFO = 20
	WARNING = 30
	ERROR = 40

	@staticmethod
	def from_str(level_str: str) -> 'Level':
		level_str = level_str.lower()
		if level_str == 'trace':
			return Level.TRACE
		elif level_str == 'debug':
			return Level.DEBUG
		elif level_str == 'info':
			return Level.INFO
		elif level_str == 'warning':
			return Level.WARNING
		elif level_str == 'error':
			return Level.ERROR
		else:
			raise ValueError(f'Unknown logging level: {level_str}')


FORMATTING_MUTEX = threading.Lock()


class Formatter(abc.ABC):
	@abc.abstractmethod
	def accepts(self, level: Level) -> bool: ...

	def log(self, level: Level, message: str, **kw) -> None:
		if self.accepts(level):
			with FORMATTING_MUTEX:
				self.dump(level, message, **kw)

	@abc.abstractmethod
	def dump(self, level: Level, message: str, **kw) -> None: ...

	def trace(self, message: str, **kw) -> None:
		self.log(Level.TRACE, message, **kw)

	def debug(self, message: str, **kw) -> None:
		self.log(Level.DEBUG, message, **kw)

	def info(self, message: str, **kw) -> None:
		self.log(Level.INFO, message, **kw)

	def warning(self, message: str, **kw) -> None:
		self.log(Level.WARNING, message, **kw)

	def error(self, message: str, **kw) -> None:
		self.log(Level.ERROR, message, **kw)

	def with_keys(self, keys: dict) -> 'Formatter':
		return _WithKeysFormatter(self, keys)


class _WithKeysFormatter(Formatter):
	keys: dict

	def __init__(self, parent: Formatter, keys: dict):
		if isinstance(parent, _WithKeysFormatter):
			self.keys = {**parent.keys, **keys}
			self.parent = parent.parent
		else:
			self.keys = keys
			self.parent = parent

	def log(self, level: str, msg: str, **kwargs) -> None:
		self.parent.log(level, msg, **self.keys, **kwargs)

	def with_keys(self, keys: dict) -> Formatter:
		return _WithKeysFormatter(self.parent, {**self.keys, **keys})


class NoFormatter(Formatter):
	def log(self, level: str, msg: str, **kwargs) -> None:
		pass

	def with_keys(self, keys: dict) -> Formatter:
		return self


class TextFormatter(Formatter):
	def __init__(self, file: io.TextIOBase, min_level: Level = Level.INFO):
		self.file = file
		self.min_level = min_level

	def accepts(self, level: Level) -> bool:
		return level.value >= self.min_level.value

	def dump(self, level: Level, message: str, **kw) -> None:
		print(f'[{level.name.upper()}] {message}', file=self.file)
		for k, v in kw.items():
			print(f'  {k}: {v}', file=self.file)


def _log_unwrap(x, seen: set[int] | None = None):
	if seen is None:
		seen = set()

	# Simple types
	if x is None:
		return None
	if isinstance(x, (str, int, float, bool)):
		return x
	if isinstance(x, Path):
		return str(x)
	if isinstance(x, bytes):
		return x.hex()
	if isinstance(x, enum.Enum):
		return x.name

	# Detailed exception handling with traceback
	if isinstance(x, BaseException):
		tb = traceback.format_exception(x)
		return _log_unwrap(
			{
				'message': x.args[0] if len(x.args) == 1 else x.args,
				'type': x.__class__.__name__,
				'notes': getattr(x, '__notes__', []),
				'traceback': tb,
			},
			seen,
		)

	# Circular reference detection
	x_id = id(x)
	if x_id in seen:
		return f'<circular:{x_id}>'
	seen.add(x_id)

	try:
		if isinstance(x, collections.abc.Mapping):
			return {k: _log_unwrap(v, seen) for k, v in x.items()}
		if isinstance(x, (set, frozenset)):
			r = list(x)
			try:
				r.sort()
			except Exception:
				pass
			return [_log_unwrap(v, seen) for v in r]
		if isinstance(x, collections.abc.Iterable):
			return [_log_unwrap(v, seen) for v in x]

		return repr(x)
	finally:
		seen.remove(x_id)


class JsonFormatter(Formatter):
	def __init__(self, file: io.TextIOBase, min_level: Level = Level.INFO):
		self.file = file
		self.min_level = min_level

	def accepts(self, level: Level) -> bool:
		return level.value >= self.min_level.value

	def dump(self, level: Level, message: str, **kw) -> None:
		json.dump(
			{
				'level': level.name,
				'message': message,
				**_log_unwrap(kw),
			},
			self.file,
		)

		self.file.write('\n')
		self.file.flush()
