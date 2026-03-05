"""Ya Test Runner - A Python test runner utility."""

__version__ = '0.0.1'

__all__ = (
	'SharedContext',
	'const',
	'exec',
	'test',
	'stage',
	'util',
	'formatter',
)

import json
from dataclasses import dataclass, field
from pathlib import Path
import subprocess
import threading
from typing import Any
from . import formatter
from ya_test_runner.formatter import Formatter, Sink
from .util.watchdog import Watchdog


@dataclass
class SharedContext:
	root_dir: Path
	logger: Formatter
	printer: Sink
	watchdog: Watchdog = field(default_factory=Watchdog.start)

	_git_files: list[Path] | None = None
	_interrupted: threading.Event = field(default_factory=threading.Event)
	_config: dict[str, Any] | None = None

	@property
	def config(self) -> dict[str, Any]:
		"""Load config from .ya-test.json if it exists."""
		if self._config is not None:
			return self._config

		config_path = self.root_dir / '.ya-test.json'
		if config_path.exists():
			conf = json.loads(config_path.read_text())
		else:
			conf = {}
		self._config = conf
		return conf

	@property
	def artifacts_dir(self) -> Path:
		"""Get the artifacts directory from config, or default to root_dir/build/test-artifacts."""
		artifacts_path = self.config.get('artifacts_dir')
		if artifacts_path:
			path = Path(artifacts_path)
			if not path.is_absolute():
				path = self.root_dir / path
			return path
		return self.root_dir / 'build' / 'test-artifacts'

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
