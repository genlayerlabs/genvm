import abc
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
import typing

from . import exec

import ya_test_runner


class Description(typing.NamedTuple):
	name: str
	needed_services: frozenset['ya_test_runner.stage.collection.Semaphore'] = frozenset()
	tags: frozenset[str] = frozenset()
	console_pool: bool = False

	def with_tags(self, new_tags: typing.Iterable[str]) -> 'Description':
		return self._replace(tags=self.tags.union(new_tags))


@dataclass
class Result:
	passed: bool
	context: dict[str, typing.Any]
	elapsed_seconds: float
	retries: int | None = None


class Case(metaclass=abc.ABCMeta):
	description: Description

	@abc.abstractmethod
	async def into_steps(self) -> list[exec.step.Step]: ...


class CommandToResultStep(exec.step.Python):
	def to_str(self):
		return '<command result -> test result>'

	async def run(self, previous_results: list[typing.Any]) -> Result:
		assert len(previous_results) > 0
		res = previous_results[-1]
		assert isinstance(res, exec.command.Result)

		return Result(
			passed=res.exit_code == 0,
			context={
				'stdout': res.stdout,
				'stderr': res.stderr,
			},
			elapsed_seconds=res.elapsed_seconds,
		)


class ResultStopIfErrorStep(exec.step.Python):
	def to_str(self):
		return '<test result -> raise if error>'

	async def run(self, previous_results: list[typing.Any]) -> None:
		assert len(previous_results) > 0
		res = previous_results[-1]
		assert isinstance(res, Result)

		if not res.passed:
			raise FinishedEarlyException(result=res)


@dataclass
class FinishedEarlyException(Exception):
	result: Result


@dataclass
class StepsCase(Case):
	description: Description
	steps: Sequence[exec.step.Step]

	async def into_steps(self) -> list[exec.step.Step]:
		return list(self.steps)


@dataclass
class SimpleCommandCase(Case):
	description: Description
	env: Mapping[str, str | Path]
	cwd: Path
	command: list[str | Path]
	mode: exec.command.RunMode = exec.command.RunMode.SILENT

	async def into_steps(self) -> list[exec.step.Step]:
		steps = []
		steps.append(exec.step.SetCwd(path=self.cwd))
		for k, v in self.env.items():
			steps.append(exec.step.SetEnv(key=k, value=v))

		steps.append(
			exec.step.Run(
				args=self.command,
				mode=self.mode,
			)
		)

		steps.append(CommandToResultStep())
		return steps


async def _OkResult(_):
	return Result(passed=True, context={}, elapsed_seconds=0, retries=None)


CONST_PASSED = exec.step.PythonFunction(_OkResult)
