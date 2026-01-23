import abc
import collections.abc
import enum
import io
import json
from dataclasses import dataclass, field
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


class Sink(abc.ABC):
	@abc.abstractmethod
	def put(self, main_topic: str, **kv) -> None: ...


class Formatter(abc.ABC):
	Level = Level

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

	def log(self, level: Level, message: str, **kwargs) -> None:
		self.parent.log(level, message, **self.keys, **kwargs)

	def with_keys(self, keys: dict) -> Formatter:
		return _WithKeysFormatter(self.parent, {**self.keys, **keys})


class NoFormatter(Formatter):
	def log(self, level: Level, message: str, **kwargs) -> None:
		pass

	def with_keys(self, keys: dict) -> Formatter:
		return self


def _is_small(x) -> bool:
	if x is None:
		return False
	if isinstance(x, (int, bool)):
		return True
	if isinstance(x, str):
		return '\n' not in x
	if isinstance(x, collections.abc.Sized):
		return len(x) == 0
	if isinstance(x, BaseException):
		return False
	as_str = str(x)
	return len(as_str) < 128 and '\n' not in as_str


def _small_to_str(x) -> str:
	if x is None:
		return 'null'
	if isinstance(x, str):
		if len(x) == 0:
			return "''"
		if "'" in x:
			return repr(x)
		return f"'{x}'"
	if isinstance(x, collections.abc.Iterable) and len(x) == 0:
		return '[]'
	if isinstance(x, collections.abc.Mapping) and len(x) == 0:
		return '{}'
	return str(x)


class TextFormatter(Formatter, Sink):
	def __init__(self, file: io.TextIOBase, min_level: Level = Level.INFO):
		self.file = file
		self.min_level = min_level

	def accepts(self, level: Level) -> bool:
		return level.value >= self.min_level.value

	def _do_dump(self, ind, data):
		if isinstance(data, (set, frozenset, BaseException)):
			data = _log_unwrap(data)

		if isinstance(data, collections.abc.Mapping):
			for k, v in data.items():
				self.file.write('  ' * ind)
				if _is_small(v):
					self.file.write(k)
					self.file.write(': ')
					self.file.write(str(v))
					self.file.write('\n')
				elif isinstance(v, collections.abc.Iterable) and _is_small(k):
					self.file.write(f'{k}:\n')
					self._do_dump(ind + 1, v)
				else:
					self.file.write(f'=== {k} === \n')
					self._do_dump(ind + 1, v)
		elif isinstance(data, str):
			if '\n' in data:
				for line in data.splitlines():
					self.file.write('  ' * ind)
					self.file.write(line.rstrip())
					self.file.write('\n')
			else:
				self.file.write('  ' * ind)
				self.file.write(repr(data))
				self.file.write('\n')
		elif isinstance(data, collections.abc.Iterable):
			for item in data:
				if _is_small(item):
					self.file.write('  ' * ind + '- ')
					self.file.write(_small_to_str(item))
					self.file.write('\n')
				else:
					self.file.write('  ' * ind)
					self.file.write('-\n')
					self._do_dump(ind + 1, item)
		else:
			self.file.write('  ' * ind)
			self.file.write(_small_to_str(data))
			self.file.write('\n')

	def put(self, main_topic: str, **kv):
		self.file.write(main_topic)
		self.file.write('\n')
		self._do_dump(1, kv)
		self.file.flush()
		pass

	def dump(self, level: Level, message: str, **kw) -> None:
		self.put(f'[{level.name.upper()}] {message}', **kw)


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

	if isinstance(x, (set, frozenset)):
		r = list(x)
		try:
			r.sort()
		except Exception:
			pass
		return [_log_unwrap(v, seen) for v in r]

	# Detailed exception handling with traceback
	if isinstance(x, BaseException):
		tb = traceback.format_exception(x)
		return _log_unwrap(
			{
				'message': x.args[0] if len(x.args) == 1 else x.args,
				'type': x.__class__.__name__,
				'notes': getattr(x, '__notes__', []),
				'traceback': [f.rstrip() for f in tb],
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


class JsonFormatter(Formatter, Sink):
	def __init__(self, file: io.TextIOBase, min_level: Level = Level.INFO):
		self.file = file
		self.min_level = min_level

	def accepts(self, level: Level) -> bool:
		return level.value >= self.min_level.value

	def put(self, main_topic: str, **kv):
		dct = {}
		if level := kv.get('level'):
			dct['level'] = level
		dct['message'] = main_topic
		dct.update(_log_unwrap(kv))
		json.dump(
			dct,
			self.file,
		)

		self.file.write('\n')
		self.file.flush()

	def dump(self, level: Level, message: str, **kw) -> None:
		self.put(message, level=level, **kw)


@dataclass
class BoxplotData:
	"""Data for rendering a boxplot."""

	lower_whisker: float
	q1: float
	median: float
	q3: float
	upper_whisker: float
	outliers_low: list[float] = field(default_factory=list)
	outliers_high: list[float] = field(default_factory=list)

	@classmethod
	def from_points(cls, points: list[float]) -> 'BoxplotData':
		"""
		Create BoxplotData from a list of data points.

		Calculates quartiles, whiskers (1.5 * IQR), and identifies outliers.
		"""
		if len(points) == 0:
			raise ValueError('Cannot create boxplot from empty list')

		sorted_pts = sorted(points)
		n = len(sorted_pts)

		def percentile(p: float) -> float:
			idx = (n - 1) * p
			lo = int(idx)
			hi = min(lo + 1, n - 1)
			frac = idx - lo
			return sorted_pts[lo] * (1 - frac) + sorted_pts[hi] * frac

		q1 = percentile(0.25)
		median = percentile(0.5)
		q3 = percentile(0.75)

		iqr = q3 - q1
		lower_fence = q1 - 1.5 * iqr
		upper_fence = q3 + 1.5 * iqr

		outliers_low = [x for x in sorted_pts if x < lower_fence]
		outliers_high = [x for x in sorted_pts if x > upper_fence]

		# Whiskers extend to the most extreme non-outlier values
		non_outliers = [x for x in sorted_pts if lower_fence <= x <= upper_fence]
		lower_whisker = non_outliers[0] if non_outliers else q1
		upper_whisker = non_outliers[-1] if non_outliers else q3

		return cls(
			lower_whisker=lower_whisker,
			q1=q1,
			median=median,
			q3=q3,
			upper_whisker=upper_whisker,
			outliers_low=outliers_low,
			outliers_high=outliers_high,
		)

	def render(self, width: int = 40) -> str:
		"""
		Render the boxplot as ASCII art.

		Example output:
		```
		*  |---[===X==]---| * *
		|  |   |   |  |   |   |
		|  |   |   |  |   |   10
		|  |   |   |  |   8
		|  |   |   |  7
		|  |   |   5
		|  |   3
		|  2
		0
		```
		"""
		# Collect all values with their symbols
		points: list[tuple[float, str]] = []

		for v in sorted(self.outliers_low):
			points.append((v, '*'))
		points.append((self.lower_whisker, '|'))
		points.append((self.q1, '['))
		points.append((self.median, 'X'))
		points.append((self.q3, ']'))
		points.append((self.upper_whisker, '|'))
		for v in sorted(self.outliers_high):
			points.append((v, '*'))

		if len(points) < 2:
			return ''

		min_val = points[0][0]
		max_val = points[-1][0]
		val_range = max_val - min_val

		if val_range == 0:
			val_range = 1

		def val_to_pos(v: float) -> int:
			return round((v - min_val) / val_range * (width - 1))

		# Build positions list with (position, value, symbol)
		positioned: list[tuple[int, float, str]] = []
		for val, sym in points:
			pos = val_to_pos(val)
			positioned.append((pos, val, sym))

		# Build the top line
		top_line = [' '] * width

		# Fill in whisker lines
		lw_pos = val_to_pos(self.lower_whisker)
		q1_pos = val_to_pos(self.q1)
		q3_pos = val_to_pos(self.q3)
		uw_pos = val_to_pos(self.upper_whisker)

		# Whisker dashes
		for i in range(lw_pos + 1, q1_pos):
			top_line[i] = '-'
		for i in range(uw_pos - 1, q3_pos, -1):
			top_line[i] = '-'

		# Box equals
		for i in range(q1_pos + 1, q3_pos):
			top_line[i] = '='

		# Place symbols (may overwrite)
		for pos, val, sym in positioned:
			top_line[pos] = sym

		# Build label lines
		lines = [''.join(top_line)]

		# Sort positions right to left for label rendering
		labels_to_render = [(pos, val) for pos, val, sym in positioned]

		while labels_to_render:
			line = [' '] * width
			# Draw pipes for all remaining positions except the rightmost
			for pos, val in labels_to_render[:-1]:
				line[pos] = '|'

			# Place the rightmost label
			rightmost_pos, rightmost_val = labels_to_render[-1]
			label = self._format_value(rightmost_val)
			# Splice label into line starting at rightmost_pos
			for i, ch in enumerate(label):
				if rightmost_pos + i < width:
					line[rightmost_pos + i] = ch

			lines.append(''.join(line).rstrip())
			labels_to_render.pop()

		return '\n'.join(lines)

	@staticmethod
	def _format_value(val: float) -> str:
		"""Format a value for boxplot label."""
		if val == int(val):
			return str(int(val))
		return f'{val:.2g}'
