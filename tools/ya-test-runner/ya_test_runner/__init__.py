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
import re
from dataclasses import dataclass, field
from pathlib import Path
import subprocess
import threading
from typing import Any
from . import formatter
from ya_test_runner.formatter import Formatter, Sink
from .util.watchdog import Watchdog


_UNSAFE_NAME_RE = re.compile(r'[^A-Za-z0-9._-]+')


def _sanitize_rel_path(name: str) -> Path:
	"""
	Turn a (path-based) test name into a safe *relative* path, preserving ``/``
	as sub-directory separators and dropping any traversal components.
	"""
	parts: list[str] = []
	for raw in name.split('/'):
		part = _UNSAFE_NAME_RE.sub('_', raw).strip('_')
		if part in ('', '.', '..'):
			continue
		parts.append(part)
	if not parts:
		parts = ['unnamed']
	return Path(*parts)


@dataclass
class SharedContext:
	root_dir: Path
	logger: Formatter
	printer: Sink
	watchdog: Watchdog = field(default_factory=Watchdog.start)

	# When running a test case, this points to that case's per-test artifact
	# directory. ``None`` outside of a test case.
	case_dir: Path | None = None

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

	def case_dir_for(self, name: str) -> Path:
		"""
		Per-test artifact directory: ``<artifacts_dir>/cases/<name>``, where
		``name`` is laid out as nested sub-directories (path-based test names
		keep their ``/`` structure rather than being flattened). Holds both the
		runner's own ``log.ytr.log`` and any artifacts the test writes.
		"""
		return self.artifacts_dir / 'cases' / _sanitize_rel_path(name)

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
