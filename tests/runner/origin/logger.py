import abc
import json
import sys


class Logger(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	def log(self, level: str, msg: str, **kwargs) -> None: ...

	def trace(self, msg: str, **kwargs) -> None:
		self.log('trace', msg, **kwargs)

	def debug(self, msg: str, **kwargs) -> None:
		self.log('debug', msg, **kwargs)

	def info(self, msg: str, **kwargs) -> None:
		self.log('info', msg, **kwargs)

	def warning(self, msg: str, **kwargs) -> None:
		self.log('warning', msg, **kwargs)

	def error(self, msg: str, **kwargs) -> None:
		self.log('error', msg, **kwargs)


class NoLogger(Logger):
	def log(self, level: str, msg: str, **kwargs) -> None:
		pass


_level_to_num = {
	'trace': 10,
	'debug': 20,
	'info': 30,
	'warning': 40,
	'error': 50,
}


class StderrLogger(Logger):
	def __init__(self, min_level: str = 'info'):
		self.min_level = _level_to_num[min_level]

	def log(self, level: str, msg: str, **kwargs) -> None:
		if _level_to_num[level] < self.min_level:
			return
		json.dump(
			{
				'message': msg,
				'level': level,
				**kwargs,
			},
			sys.stderr,
		)
