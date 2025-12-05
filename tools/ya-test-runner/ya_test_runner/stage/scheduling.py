from types import SimpleNamespace
import ya_test_runner
from ya_test_runner import SharedContext


from .collection import Env as CollectionEnv
import typing


class StartCases(typing.NamedTuple):
	id: int
	cases: list[ya_test_runner.test.Case]


class AwaitAllCases(typing.NamedTuple):
	id: int


type Action = StartCases | AwaitAllCases


class Env(typing.NamedTuple):
	actions: list[Action]
	args: SimpleNamespace


def run(shared: SharedContext, collection_env: CollectionEnv) -> Env:
	next_id = 1
	parallel_batch = StartCases(0, [])
	actions: list[Action] = []
	for case in collection_env.cases:
		if len(case.description.needed_services) > 0:
			raise NotImplementedError('Service scheduling is not implemented yet')
		if case.description.console_pool:
			actions.append(
				StartCases(
					id=next_id,
					cases=[case],
				)
			)
			actions.append(
				AwaitAllCases(
					id=next_id,
				)
			)
			next_id += 1
		else:
			parallel_batch.cases.append(case)
	if len(parallel_batch.cases) > 0:
		actions.append(parallel_batch)
		actions.append(
			AwaitAllCases(
				id=parallel_batch.id,
			)
		)

	return Env(
		actions=actions,
		args=collection_env.args,
	)
