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


def _validate_test_dependencies(
	cases: list[ya_test_runner.test.Case],
) -> dict[str, ya_test_runner.test.Case]:
	"""
	Validate test-to-test dependencies: check for circular deps and return cases_by_name.
	Dependencies on names not in the current set are silently ignored (e.g. filtered out).
	"""
	cases_by_name: dict[str, ya_test_runner.test.Case] = {}
	for case in cases:
		cases_by_name[case.description.name] = case

	# Detect circular dependencies via DFS
	visited: set[str] = set()
	in_stack: set[str] = set()

	def visit(name: str) -> None:
		if name in visited:
			return
		if name in in_stack:
			raise ValueError(f'Circular test dependency detected involving {name}')
		in_stack.add(name)
		case = cases_by_name.get(name)
		if case is not None:
			for dep_name in case.description.depends_on:
				if dep_name in cases_by_name:
					visit(dep_name)
		in_stack.remove(name)
		visited.add(name)

	for name in cases_by_name:
		visit(name)

	return cases_by_name


def _get_effective_services(
	case: ya_test_runner.test.Case,
	cases_by_name: dict[str, ya_test_runner.test.Case],
	_memo: dict[str, frozenset[str]] | None = None,
) -> frozenset[str]:
	"""
	Compute effective service names for a test case: its own services plus
	services of all transitive test dependencies. This ensures a dependent
	test is never placed in an earlier batch than its dependency.
	"""
	if _memo is None:
		_memo = {}

	name = case.description.name
	if name in _memo:
		return _memo[name]

	names: set[str] = set()
	# Own services
	for svc in case.description.needed_services:
		names.add(svc.name)
		if svc.depends_on:
			for dep in svc.depends_on:
				names.add(dep.name)

	# Transitive test dependency services
	for dep_name in case.description.depends_on:
		dep_case = cases_by_name.get(dep_name)
		if dep_case is not None:
			names.update(_get_effective_services(dep_case, cases_by_name, _memo))

	result = frozenset(names)
	_memo[name] = result
	return result


def run(shared: SharedContext, collection_env: CollectionEnv) -> Env:
	next_id = 1
	actions: list[Action] = []

	# Track running batches: batch_id -> set of service names the batch depends on
	running_batches: dict[int, set[str]] = {}

	# Validate test dependencies and build name lookup
	cases_by_name = _validate_test_dependencies(collection_env.cases)

	# Collect all services needed by all cases (using effective services)
	all_needed_services: set[Service] = set()
	for case in collection_env.cases:
		# Collect direct services
		for svc in case.description.needed_services:
			all_needed_services.add(svc)
			if svc.depends_on:
				for dep in svc.depends_on:
					all_needed_services.add(dep)
		# Also collect services from test dependencies (transitively)
		for dep_name in case.description.depends_on:
			dep_case = cases_by_name.get(dep_name)
			if dep_case is not None:
				for svc in dep_case.description.needed_services:
					all_needed_services.add(svc)
					if svc.depends_on:
						for dep in svc.depends_on:
							all_needed_services.add(dep)

	# Topo sort: dependencies first, dependents last
	topo_sorted_services = _topo_sort_services(all_needed_services)

	# Use effective services for batch placement
	effective_services_memo: dict[str, frozenset[str]] = {}

	def get_effective_service_names(case: ya_test_runner.test.Case) -> frozenset[str]:
		return _get_effective_services(case, cases_by_name, effective_services_memo)

	# Separate cases by whether they have effective services
	cases_without_services: list[ya_test_runner.test.Case] = []
	cases_with_services: list[ya_test_runner.test.Case] = []

	for case in collection_env.cases:
		if len(get_effective_service_names(case)) > 0:
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

		# Find cases that can now run (all their effective services are active)
		ready_cases: list[ya_test_runner.test.Case] = []
		still_waiting: list[ya_test_runner.test.Case] = []

		for case in remaining_cases:
			required_services = get_effective_service_names(case)
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
				batch_services.update(get_effective_service_names(case))
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
