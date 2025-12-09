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

from dataclasses import dataclass
from pathlib import Path
import subprocess
from ya_test_runner.formatter import Formatter, Sink


@dataclass
class SharedContext:
	root_dir: Path
	logger: Formatter
	printer: Sink

	_git_files: list[Path] | None = None

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


from . import const, exec, test, stage, util
