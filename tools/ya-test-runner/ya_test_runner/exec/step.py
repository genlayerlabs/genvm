import abc
from pathlib import Path
from dataclasses import dataclass
import shlex
import typing

from ya_test_runner import SharedContext

from . import command


@dataclass
class SetCwd:
	path: Path

	def to_str(self) -> str:
		return f'cd {shlex.quote(str(self.path))}'


@dataclass
class SetEnv:
	key: str
	value: str | Path

	def to_str(self) -> str:
		return f'export {self.key}={shlex.quote(str(self.value))}'


@dataclass
class Run:
	args: list[str | Path]
	mode: command.RunMode

	def to_str(self) -> str:
		cmd = ' '.join(shlex.quote(str(arg)) for arg in self.args)
		return cmd


class Python(metaclass=abc.ABCMeta):
	@abc.abstractmethod
	async def run(self, previous_results: list[typing.Any]) -> typing.Any: ...

	def to_str(self) -> str:
		return f'# <python step>'


class PythonFunction(Python):
	def __init__(
		self, func: typing.Callable[[list[typing.Any]], typing.Awaitable[typing.Any]]
	):
		self.func = func

	async def run(self, previous_results: list[typing.Any]) -> typing.Any:
		return await self.func(previous_results)


type Step = SetCwd | SetEnv | Run | Python


def optimize_steps(steps: list[Step]) -> list[Step]:
	has_effect = [False] * len(steps)
	last_cwd_idx: int | None = None
	last_env: dict[str, tuple[int, str | Path]] = {}
	for i, step in enumerate(steps):
		if isinstance(step, SetCwd):
			if last_cwd_idx is None or steps[last_cwd_idx].path != step.path:
				last_cwd_idx = i
		elif isinstance(step, SetEnv):
			if step.key not in last_env or last_env[step.key][1] != step.value:
				last_env[step.key] = (i, step.value)
		elif isinstance(step, Run):
			has_effect[i] = True  # run always has effect
			if last_cwd_idx is not None:
				has_effect[last_cwd_idx] = True
			for k, (idx, _v) in last_env.items():
				has_effect[idx] = True
		elif isinstance(step, Python):
			# we assume python steps don't have side effects
			has_effect[i] = True
		else:
			raise ValueError(f'Unknown step type: {step!r}')
	return [s for i, s in enumerate(steps) if has_effect[i]]


def dump_steps(steps: list[Step]) -> list[str]:
	return [step.to_str() for step in steps]


async def run_steps(ctx: SharedContext, steps: list[Step]) -> list[typing.Any]:
	results = []
	env = {}
	cwd = Path.cwd()
	for s in steps:
		if isinstance(s, Python):
			results.append(await s.run(results))
		elif isinstance(s, SetCwd):
			cwd = s.path
		elif isinstance(s, SetEnv):
			env[s.key] = str(s.value)
		elif isinstance(s, Run):
			cmd = command.Command(s.args, cwd, env)
			results.append(
				await cmd.run(
					ctx=ctx,
					mode=s.mode,
				)
			)
		else:
			raise ValueError(f'Unknown step type: {s!r}')
	return results
