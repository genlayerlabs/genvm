import argparse
import contextlib
from copy import copy
from pathlib import Path
import sys
from types import SimpleNamespace
import typing

import ya_test_runner

from ya_test_runner import const
from ya_test_runner import SharedContext


def _check_relative_id(relative: str) -> list[str]:
	components = relative.split('/')
	for c in components:
		if c in ('', '.', '..'):
			raise ValueError(f'Invalid component in path: {c}')
		if '/' in c or '\\' in c:
			raise ValueError(f'Invalid component in path: {c}')
	return components


type Collector = typing.Callable[['ya_test_runner.stage.collection.Context'], None]
type Reporter = typing.Callable[
	['ya_test_runner.SharedContext', 'ya_test_runner.stage.execution.Env'], None
]


class Context:
	shared: SharedContext
	parser: argparse.ArgumentParser
	run_parser: argparse.ArgumentParser
	shared_parser: argparse.ArgumentParser
	filter_parser: argparse.ArgumentParser
	current_path: Path
	plugins: dict[str, typing.Any]

	_collectors: list[Collector]
	_reporter: list[Reporter]

	def register_plugin(self):
		raise NotImplementedError()

	def add_reporter(self, step: Reporter) -> None:
		"""Register a step to run after test execution completes."""
		self._reporter.append(step)

	def eval_file(self, relative: str) -> None:
		components = _check_relative_id(relative)
		dir_components = components[:-1]
		new_ctx = copy(self)
		new_ctx.current_path = self.current_path.joinpath(*dir_components)
		with with_context(new_ctx) as ctx:
			ctx._eval_file(ctx.current_path.joinpath(components[-1]))

	def add_dir(self, relative: str) -> None:
		components = _check_relative_id(relative)
		new_ctx = copy(self)
		new_ctx.current_path = self.current_path.joinpath(*components)
		with with_context(new_ctx) as ctx:
			ctx._eval_file(ctx.current_path.joinpath(const.ROOT_FILE_NAME))

	def add_collector(self, collector: Collector) -> None:
		self._collectors.append(collector)

	def _eval_file(self, file: Path) -> None:
		rel_path = file.relative_to(self.shared.root_dir)
		as_module = rel_path.with_suffix('').as_posix().replace('/', '.')
		import types

		module = types.ModuleType(as_module)
		module.__dict__['__file__'] = str(file.absolute())
		self.shared.logger.debug('evaluating include dir', include_file=file)
		compiled = compile(file.read_text(), str(file.absolute()), 'exec')
		exec(compiled, module.__dict__)
		sys.modules[as_module] = module


_GLOBAL_CTX: Context | None = None


def current_context() -> Context:
	if _GLOBAL_CTX is None:
		raise RuntimeError('No global context is set')
	return _GLOBAL_CTX


@contextlib.contextmanager
def with_context(ctx: Context) -> typing.Generator[Context, None, None]:
	global _GLOBAL_CTX
	old_ctx = _GLOBAL_CTX
	try:
		_GLOBAL_CTX = ctx
		yield ctx
	finally:
		_GLOBAL_CTX = old_ctx


class Env(typing.NamedTuple):
	plugins: SimpleNamespace
	args: argparse.Namespace
	collectors: list[Collector]
	post_run_steps: list[Reporter]


class InitialEnv(typing.NamedTuple):
	parser: argparse.ArgumentParser
	run_parser: argparse.ArgumentParser
	shared_parser: argparse.ArgumentParser
	filter_parser: argparse.ArgumentParser
	remaining_args: list[str]


def run(shared: SharedContext, env: InitialEnv, /) -> Env:
	ctx = Context()
	ctx.shared = shared
	ctx.parser = env.parser
	ctx.run_parser = env.run_parser
	ctx.shared_parser = env.shared_parser
	ctx.filter_parser = env.filter_parser
	ctx.plugins = {}
	ctx._collectors = []
	ctx._reporter = []
	ctx.current_path = shared.root_dir
	with with_context(ctx) as ctx:
		ctx.eval_file(const.ROOT_FILE_NAME)

	return Env(
		args=ctx.parser.parse_args(env.remaining_args),
		plugins=SimpleNamespace(**ctx.plugins),
		collectors=ctx._collectors,
		post_run_steps=ctx._reporter,
	)
