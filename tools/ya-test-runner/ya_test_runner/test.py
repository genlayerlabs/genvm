import abc
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
import time
import typing

from . import exec

import ya_test_runner


class Description(typing.NamedTuple):
	name: str
	needed_services: frozenset['ya_test_runner.stage.collection.Service'] = frozenset()
	tags: frozenset[str] = frozenset()
	console_pool: bool = False

	def with_tags(self, new_tags: typing.Iterable[str]) -> 'Description':
		return self._replace(tags=self.tags.union(new_tags))

	def with_services(
		self, services: typing.Iterable['ya_test_runner.stage.collection.Service']
	) -> 'Description':
		return self._replace(needed_services=self.needed_services.union(services))


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

	async def run(self, previous_results: list[typing.Any]):
		assert len(previous_results) > 0
		res = previous_results[-1]
		assert isinstance(res, Result)

		if not res.passed:
			raise FinishedEarlyException(result=res)

		return res


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


@dataclass
class _BenchMeasureData:
	stamp: float


class BenchMeasureStep(exec.step.Python):
	def to_str(self):
		return '<benchmark: time point>'

	async def run(self, _previous_results: list[typing.Any]):
		return _BenchMeasureData(time.perf_counter())


class BenchCollectStep(exec.step.Python):
	def __init__(self, printer: ya_test_runner.formatter.Sink, **kv):
		self._kv = kv
		self._printer = printer

	def to_str(self):
		return '<benchmark: report>'

	async def run(self, previous_results: list[typing.Any]):
		ended_stamp = time.perf_counter()
		measurements: list[float] = []
		last_good = 0
		for res in previous_results:
			if isinstance(res, _BenchMeasureData):
				measurements.append(res.stamp)
			else:
				previous_results[last_good] = res
				last_good += 1
		previous_results = previous_results[:last_good]
		measurements.append(ended_stamp)

		bench_res = [x - y for y, x in zip(measurements, measurements[1:])]

		import ya_test_runner.formatter as formatter

		bp = formatter.BoxplotData.from_points(bench_res)

		self._printer.put(
			'Benchmark results', **self._kv, plot=bp.render(), results=bench_res
		)


CONST_PASSED = exec.step.PythonFunction(_OkResult)
