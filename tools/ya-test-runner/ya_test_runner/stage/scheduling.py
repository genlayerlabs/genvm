from types import SimpleNamespace
import ya_test_runner
from ya_test_runner import SharedContext


from .collection import Env as CollectionEnv, Service, Semaphore
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


def _get_semaphore_usage(service: Service) -> dict[int, int]:
	"""Get semaphore ID -> count mapping for a service."""
	return {id(cs.semaphore): cs.count for cs in service._consumed_semaphores.values()}


def _services_conflict(
	active_services: dict[str, Service],
	new_service: Service,
	semaphore_limits: dict[int, int],
) -> bool:
	"""
	Check if starting a new service would conflict with active services.

	A conflict occurs when the total semaphore usage would exceed the limit.
	"""
	# Calculate current usage
	current_usage: dict[int, int] = {}
	for svc in active_services.values():
		for sem_id, count in _get_semaphore_usage(svc).items():
			current_usage[sem_id] = current_usage.get(sem_id, 0) + count

	# Check if adding new service would exceed limits
	for sem_id, count in _get_semaphore_usage(new_service).items():
		total = current_usage.get(sem_id, 0) + count
		limit = semaphore_limits.get(sem_id, 1)
		if total > limit:
			return True

	return False


def _find_conflicting_services(
	active_services: dict[str, Service],
	new_service: Service,
	semaphore_limits: dict[int, int],
) -> list[Service]:
	"""
	Find which active services conflict with the new service.
	"""
	new_usage = _get_semaphore_usage(new_service)
	conflicting = []

	for svc in active_services.values():
		svc_usage = _get_semaphore_usage(svc)
		# Check if they share any semaphores
		for sem_id in new_usage:
			if sem_id in svc_usage:
				# They share a semaphore - check if combined usage exceeds limit
				combined = new_usage[sem_id] + svc_usage[sem_id]
				limit = semaphore_limits.get(sem_id, 1)
				if combined > limit:
					conflicting.append(svc)
					break

	return conflicting


def _get_services_with_dependencies(service: Service) -> list[Service]:
	"""
	Get a list of services to start, including dependencies, in the correct order.
	Dependencies come before the services that depend on them.
	"""
	result: list[Service] = []
	visited: set[str] = set()

	def visit(svc: Service) -> None:
		if svc.name in visited:
			return
		visited.add(svc.name)

		# First visit dependencies
		if svc.depends_on:
			for dep in svc.depends_on:
				visit(dep)

		result.append(svc)

	visit(service)
	return result


def _get_all_services_with_dependencies(services: list[Service]) -> list[Service]:
	"""
	Get all services including their dependencies in the correct start order.
	"""
	result: list[Service] = []
	visited: set[str] = set()

	for svc in services:
		for s in _get_services_with_dependencies(svc):
			if s.name not in visited:
				visited.add(s.name)
				result.append(s)

	return result


def run(shared: SharedContext, collection_env: CollectionEnv) -> Env:
	next_id = 1
	actions: list[Action] = []

	# Track running batches: batch_id -> set of service names the batch depends on
	running_batches: dict[int, set[str]] = {}

	def await_batches_using_service(service_name: str) -> None:
		"""Await all running batches that depend on a service before stopping it."""
		nonlocal running_batches
		to_await = [
			batch_id
			for batch_id, services in running_batches.items()
			if service_name in services
		]
		for batch_id in to_await:
			actions.append(AwaitAllCases(id=batch_id))
			del running_batches[batch_id]

	def await_all_running_batches() -> None:
		"""Await all currently running batches."""
		nonlocal running_batches
		for batch_id in list(running_batches.keys()):
			actions.append(AwaitAllCases(id=batch_id))
		running_batches.clear()

	# Build semaphore limits map
	semaphore_limits: dict[int, int] = {}
	for sem in collection_env.semaphores:
		semaphore_limits[id(sem)] = sem.limit

	# Separate cases by whether they need services
	cases_without_services: list[ya_test_runner.test.Case] = []
	cases_with_services: list[ya_test_runner.test.Case] = []

	for case in collection_env.cases:
		if len(case.description.needed_services) > 0:
			cases_with_services.append(case)
		else:
			cases_without_services.append(case)

	# Schedule cases without services first
	parallel_batch = StartCases(0, [])
	for case in cases_without_services:
		if case.description.console_pool:
			# Console pool cases must run sequentially
			await_all_running_batches()
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
		running_batches[parallel_batch.id] = set()  # No service dependencies

	# Group cases with services by their needed_services set
	service_groups: dict[frozenset[str], list[ya_test_runner.test.Case]] = {}
	for case in cases_with_services:
		# Use service names as the key for grouping
		service_key = frozenset(svc.name for svc in case.description.needed_services)
		if service_key not in service_groups:
			service_groups[service_key] = []
		service_groups[service_key].append(case)

	# Sort groups by number of services (fewer first)
	sorted_groups = sorted(service_groups.items(), key=lambda x: len(x[0]))

	# Track active services
	active_services: dict[str, Service] = {}

	for service_names, cases in sorted_groups:
		# Get the actual Service objects from the cases
		needed_services: dict[str, Service] = {}
		for case in cases:
			for svc in case.description.needed_services:
				needed_services[svc.name] = svc

		# Get all services including dependencies in start order
		all_needed_services = _get_all_services_with_dependencies(
			list(needed_services.values())
		)

		# Check which services need to be started (not already active)
		services_to_start: list[Service] = []
		for svc in all_needed_services:
			if svc.name not in active_services:
				services_to_start.append(svc)

		# For each service to start, check for conflicts and stop conflicting services
		for new_svc in services_to_start:
			conflicting = _find_conflicting_services(
				active_services, new_svc, semaphore_limits
			)
			for conf_svc in conflicting:
				# Must await batches using this service before stopping it
				await_batches_using_service(conf_svc.name)
				actions.append(StopService(service=conf_svc))
				del active_services[conf_svc.name]

		# Start needed services that aren't already active (in dependency order)
		for svc in services_to_start:
			actions.append(StartService(service=svc))
			active_services[svc.name] = svc

		# Collect service names for this batch (including dependencies)
		batch_service_names = {svc.name for svc in all_needed_services}

		# Schedule cases in this group
		group_parallel_batch = StartCases(next_id, [])
		next_id += 1

		for case in cases:
			if case.description.console_pool:
				# Console pool cases must run sequentially
				await_all_running_batches()
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
				group_parallel_batch.cases.append(case)

		if len(group_parallel_batch.cases) > 0:
			actions.append(group_parallel_batch)
			running_batches[group_parallel_batch.id] = batch_service_names

	# Await all remaining running batches before stopping services
	await_all_running_batches()

	# Stop all remaining services at end
	for svc in active_services.values():
		actions.append(StopService(service=svc))

	return Env(
		actions=actions,
		args=collection_env.args,
	)
