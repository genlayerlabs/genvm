import json
from pathlib import Path
import subprocess
import sys


class Watchdog:
	"""Manages the watchdog subprocess that cleans up on unexpected death."""

	def __init__(self, process: subprocess.Popen):
		self._process = process

	@staticmethod
	def start() -> 'Watchdog':
		script = Path(__file__).resolve().parent / 'watchdog_main.py'
		process = subprocess.Popen(
			[sys.executable, str(script)],
			stdin=subprocess.PIPE,
			stdout=subprocess.DEVNULL,
			stderr=subprocess.DEVNULL,
		)
		return Watchdog(process)

	def add_command(self, command: list[str]) -> None:
		"""Register a cleanup command with the watchdog."""
		try:
			assert self._process.stdin is not None
			msg = json.dumps({'action': 'add', 'command': command}) + '\n'
			self._process.stdin.write(msg.encode())
			self._process.stdin.flush()
		except (BrokenPipeError, OSError):
			pass

	def remove_command(self, command: list[str]) -> None:
		"""Unregister a cleanup command from the watchdog."""
		try:
			assert self._process.stdin is not None
			msg = json.dumps({'action': 'remove', 'command': command}) + '\n'
			self._process.stdin.write(msg.encode())
			self._process.stdin.flush()
		except (BrokenPipeError, OSError):
			pass

	def stop(self) -> None:
		"""Stop the watchdog. Closes stdin to signal normal exit."""
		if self._process.stdin:
			try:
				self._process.stdin.close()
			except OSError:
				pass
		try:
			self._process.wait(timeout=5)
		except subprocess.TimeoutExpired:
			self._process.kill()
			self._process.wait()
