"""Ya Test Runner - A Python test runner utility."""

__version__ = '0.0.1'

__all__ = (
	'SharedContext',
	'const',
	'exec',
	'test',
	'stage',
	'util',
)

from dataclasses import dataclass, field
from pathlib import Path
import subprocess
import threading
from ya_test_runner.formatter import Formatter, Sink


@dataclass
class SharedContext:
	root_dir: Path
	logger: Formatter
	printer: Sink

	_git_files: list[Path] | None = None
	_interrupted: threading.Event = field(default_factory=threading.Event)

	@property
	def git_files(self) -> list[Path]:
		if self._git_files is not None:
			return self._git_files

		r = subprocess.run(
			['git', 'ls-files'],
			check=True,
			capture_output=True,
			text=True,
		)

		r = [
			self.root_dir.joinpath(x.strip())
			for x in r.stdout.splitlines()
			if x.strip() != ''
		]

		r.sort()
		self._git_files = r
		return r

	def interrupt(self) -> None:
		"""Signal that execution should be interrupted."""
		self._interrupted.set()

	@property
	def is_interrupted(self) -> bool:
		"""Check if execution has been interrupted."""
		return self._interrupted.is_set()


from . import const, exec, test, stage, util
