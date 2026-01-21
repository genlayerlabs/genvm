from types import SimpleNamespace
import ya_test_runner
from ya_test_runner import SharedContext

from .collection import Env as CollectionEnv, Service
import typing


class StartCases(typing.NamedTuple):
	id: int
	cases: list[ya_test_runner.test.Case]


class AwaitAllCases(typing.NamedTuple):
	id: int


class StartService(typing.NamedTuple):
	service: Service


class StopService(typing.NamedTuple):
	service: Service


type Action = StartCases | AwaitAllCases | StartService | StopService


class Env(typing.NamedTuple):
	actions: list[Action]
	args: SimpleNamespace


def _topo_sort_services(services: set[Service]) -> list[Service]:
	"""
	Topological sort of services via DFS.
	Returns services in dependency order: dependencies come before dependents.
	"""
	result: list[Service] = []
	visited: set[str] = set()
	in_stack: set[str] = set()

	def visit(svc: Service) -> None:
		if svc.name in visited:
			return
		if svc.name in in_stack:
			raise ValueError(f'Circular dependency detected involving {svc.name}')

		in_stack.add(svc.name)

		# Visit dependencies first
		if svc.depends_on:
			for dep in svc.depends_on:
				if dep in services:
					visit(dep)

		in_stack.remove(svc.name)
		visited.add(svc.name)
		result.append(svc)

	for svc in services:
		visit(svc)

	return result


def run(shared: SharedContext, collection_env: CollectionEnv) -> Env:
	next_id = 1
	actions: list[Action] = []

	# Track running batches: batch_id -> set of service names the batch depends on
	running_batches: dict[int, set[str]] = {}

	# Collect all services needed by all cases
	all_needed_services: set[Service] = set()
	for case in collection_env.cases:
		for svc in case.description.needed_services:
			all_needed_services.add(svc)
			# Also add dependencies
			if svc.depends_on:
				for dep in svc.depends_on:
					all_needed_services.add(dep)

	# Topo sort: dependencies first, dependents last
	topo_sorted_services = _topo_sort_services(all_needed_services)

	# Group cases by required services (as frozenset of names including dependencies)
	def get_all_service_names(case: ya_test_runner.test.Case) -> frozenset[str]:
		names: set[str] = set()
		for svc in case.description.needed_services:
			names.add(svc.name)
			if svc.depends_on:
				for dep in svc.depends_on:
					names.add(dep.name)
		return frozenset(names)

	# Separate cases by whether they need services
	cases_without_services: list[ya_test_runner.test.Case] = []
	cases_with_services: list[ya_test_runner.test.Case] = []

	for case in collection_env.cases:
		if len(case.description.needed_services) > 0:
			cases_with_services.append(case)
		else:
			cases_without_services.append(case)

	# Schedule cases without services first (they can run immediately)
	parallel_batch_no_services: list[ya_test_runner.test.Case] = []
	for case in cases_without_services:
		if case.description.console_pool:
			# Console pool: run alone, await immediately
			actions.append(StartCases(id=next_id, cases=[case]))
			actions.append(AwaitAllCases(id=next_id))
			next_id += 1
		else:
			parallel_batch_no_services.append(case)

	if parallel_batch_no_services:
		batch_id = next_id
		next_id += 1
		actions.append(StartCases(id=batch_id, cases=parallel_batch_no_services))
		running_batches[batch_id] = set()  # No service dependencies

	# Now schedule cases with services
	# Start services in topo order, start tests as soon as their services are ready
	active_services: set[str] = set()
	remaining_cases = list(cases_with_services)

	for svc in topo_sorted_services:
		# Start this service
		actions.append(StartService(service=svc))
		active_services.add(svc.name)

		# Find cases that can now run (all their services are active)
		ready_cases: list[ya_test_runner.test.Case] = []
		still_waiting: list[ya_test_runner.test.Case] = []

		for case in remaining_cases:
			required_services = get_all_service_names(case)
			if required_services.issubset(active_services):
				ready_cases.append(case)
			else:
				still_waiting.append(case)

		remaining_cases = still_waiting

		# Schedule ready cases
		parallel_batch: list[ya_test_runner.test.Case] = []
		for case in ready_cases:
			if case.description.console_pool:
				# Console pool: run alone, await immediately
				actions.append(StartCases(id=next_id, cases=[case]))
				actions.append(AwaitAllCases(id=next_id))
				next_id += 1
			else:
				parallel_batch.append(case)

		if parallel_batch:
			batch_id = next_id
			next_id += 1
			# Track which services this batch depends on
			batch_services: set[str] = set()
			for case in parallel_batch:
				batch_services.update(get_all_service_names(case))
			actions.append(StartCases(id=batch_id, cases=parallel_batch))
			running_batches[batch_id] = batch_services

	# Ending: stop services in reverse topo order (dependents first, dependencies last)
	for svc in reversed(topo_sorted_services):
		# Await all batches that depend on this service
		batches_to_await = [
			batch_id for batch_id, services in running_batches.items() if svc.name in services
		]
		for batch_id in batches_to_await:
			actions.append(AwaitAllCases(id=batch_id))
			del running_batches[batch_id]

		# Stop this service
		actions.append(StopService(service=svc))

	# Await any remaining batches (e.g., those without service dependencies)
	for batch_id in list(running_batches.keys()):
		actions.append(AwaitAllCases(id=batch_id))
	running_batches.clear()

	return Env(
		actions=actions,
		args=collection_env.args,
	)
