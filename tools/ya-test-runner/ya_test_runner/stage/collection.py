from dataclasses import dataclass
from types import SimpleNamespace
from typing import NamedTuple
from ya_test_runner import SharedContext
from .configuration import Env as ConfigurationEnv

import ya_test_runner


@dataclass
class Semaphore:
	name: str
	limit: int = 1


@dataclass
class _ConsumedSemaphore:
	semaphore: Semaphore
	count: int


@dataclass(frozen=False, eq=False)
class Service:
	name: str
	_consumed_semaphores: dict[int, _ConsumedSemaphore]
	manager: ya_test_runner.exec.service.Service
	depends_on: list['Service'] | None = None

	def __hash__(self):
		return id(self)

	def __eq__(self, other):
		return self is other


class Context:
	shared: SharedContext
	configuration: ConfigurationEnv
	_all_semaphores: list[Semaphore]
	_all_services: list[Service]
	_all_cases: list[ya_test_runner.test.Case]

	def new_semaphore(self, name: str, limit: int) -> Semaphore:
		sem = Semaphore(name=name, limit=limit)
		self._all_semaphores.append(sem)
		return sem

	def new_service(
		self,
		name: str,
		sems: list[(Semaphore, int)],
		manager: ya_test_runner.exec.service.Service,
		depends_on: list['Service'] | None = None,
	) -> Service:
		svc = Service(
			name=name, _consumed_semaphores={}, manager=manager, depends_on=depends_on
		)
		for sem, count in sems:
			svc._consumed_semaphores[id(sem)] = _ConsumedSemaphore(sem, count)
		return svc

	def add_case(self, case: ya_test_runner.test.Case):
		assert isinstance(case, ya_test_runner.test.Case)
		self._all_cases.append(case)


class Env(NamedTuple):
	cases: list[ya_test_runner.test.Case]
	semaphores: list[Semaphore]
	args: SimpleNamespace


def run(shared: SharedContext, configuration: ConfigurationEnv) -> Env:
	ctx = Context()
	ctx.shared = shared
	ctx.configuration = configuration
	ctx._all_semaphores = []
	ctx._all_services = []
	ctx._all_cases = []

	for collector in configuration.collectors:
		collector(ctx)

	return Env(
		cases=ctx._all_cases,
		semaphores=ctx._all_semaphores,
		args=configuration.args,
	)
