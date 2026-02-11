import argparse
from dataclasses import dataclass, field
from typing import NamedTuple
import typing
from ya_test_runner import SharedContext
from .configuration import Env as ConfigurationEnv

import ya_test_runner


@dataclass(frozen=False, eq=False)
class Service:
	name: str
	manager: ya_test_runner.exec.service.Service
	depends_on: list['Service'] | None = None
	meta: dict[str, typing.Any] = field(default_factory=dict)

	def __hash__(self):
		return id(self)

	def __eq__(self, other):
		return self is other


class Context:
	shared: SharedContext
	configuration: ConfigurationEnv
	_all_services: list[Service]
	_all_cases: list[ya_test_runner.test.Case]

	def new_service(
		self,
		name: str,
		manager: ya_test_runner.exec.service.Service,
		depends_on: list['Service'] | None = None,
	) -> Service:
		svc = Service(name=name, manager=manager, depends_on=depends_on)
		return svc

	def add_case(self, case: ya_test_runner.test.Case):
		assert isinstance(case, ya_test_runner.test.Case)
		self._all_cases.append(case)


class Env(NamedTuple):
	cases: list[ya_test_runner.test.Case]
	args: argparse.Namespace


def run(shared: SharedContext, configuration: ConfigurationEnv) -> Env:
	ctx = Context()
	ctx.shared = shared
	ctx.configuration = configuration
	ctx._all_services = []
	ctx._all_cases = []

	for collector in configuration.collectors:
		collector(ctx)

	return Env(
		cases=ctx._all_cases,
		args=configuration.args,
	)
