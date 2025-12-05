import argparse
import contextlib
from copy import copy
from pathlib import Path
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


class Context:
	shared: SharedContext
	parser: argparse.ArgumentParser
	current_path: Path
	plugins: dict[str, typing.Any]

	_collectors: list[Collector]

	def register_plugin(self):
		raise NotImplementedError()

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
		this_globals = {'__file__': str(file.absolute())}
		self.shared.logger.debug('evaluating include dir', include_file=file)
		compiled = compile(file.read_text(), str(file.absolute()), 'exec')
		exec(compiled, this_globals)


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


def run(
	shared: SharedContext, parser: argparse.ArgumentParser, remaining_args: list[str]
) -> Env:
	ctx = Context()
	ctx.shared = shared
	ctx.parser = parser
	ctx.plugins = {}
	ctx._collectors = []
	ctx.current_path = shared.root_dir
	with with_context(ctx) as ctx:
		ctx.eval_file(const.ROOT_FILE_NAME)

	return Env(
		args=parser.parse_args(remaining_args),
		plugins=SimpleNamespace(**ctx.plugins),
		collectors=ctx._collectors,
	)
