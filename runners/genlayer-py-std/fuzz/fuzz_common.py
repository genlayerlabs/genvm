import afl
import sys, os

from pathlib import Path
import hashlib


def do_fuzzing(target):
	real_stdin = os.fdopen(0, 'rb', closefd=False)

	def step():
		real_stdin.seek(0)

		data = real_stdin.read()
		assert isinstance(data, bytes), f'data is {type(data)}'

		target(data)

	if True:
		while afl.loop(1000):
			step()
	else:
		afl.init()
		step()


class StopFuzzingException(Exception):
	pass


class FuzzerBuilder:
	def __init__(self, buf: bytes):
		self.buf = buf

	def fetch(self, le: int) -> bytes:
		if len(self.buf) < le:
			raise StopFuzzingException()

		ret = self.buf[:le]
		self.buf = self.buf[le:]
		return ret

	def fetch_str(self) -> str:
		try:
			return self.fetch(self.fetch(1)[0]).decode('utf-8')
		except UnicodeDecodeError:
			raise StopFuzzingException()
