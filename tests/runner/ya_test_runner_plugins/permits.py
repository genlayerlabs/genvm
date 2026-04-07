"""
Permits semaphore test plugin for ya-test-runner.

Tests that the manager's permit semaphore correctly blocks new genvm executions
when all permits are exhausted.
"""

import asyncio
import pickle
import typing
from dataclasses import dataclass
from pathlib import Path

import aiohttp

import ya_test_runner
import ya_test_runner.stage.collection

import genlayer.calldata as gvm_calldata
from genlayer.types import Address

from gvm_extra.mock_host import MockHost, MockStorage
import origin.base_host as base_host

local_ctx = ya_test_runner.stage.configuration.current_context()

BUSY_CONTRACT = (
	Path(__file__).resolve().parent.parent.parent
	/ 'cases'
	/ 'stable'
	/ 'permits'
	/ 'busy_contract.py'
)

FAKE_MESSAGE: base_host.Message = {
	'contract_address': Address('0x' + '00' * 20),
	'sender_address': Address('0x' + '01' * 20),
	'origin_address': Address('0x' + '01' * 20),
	'chain_id': 0,
	'is_init': True,
}


class _TestContext(base_host.Context):
	"""Minimal Context for run_genvm calls."""

	def __init__(self, logger: base_host.Logger):
		self.logger = logger

	def on_genvm_success(self): ...
	def on_genvm_failure(self): ...
	def add_stat(self, key: str, value: typing.Any, /): ...


@dataclass
class PermitsTestCase(ya_test_runner.test.Case):
	description: ya_test_runner.test.Description
	manager_service: ya_test_runner.stage.collection.Service

	async def into_steps(self) -> list[ya_test_runner.exec.step.Step]:
		return [PermitsTestStep(self)]


class PermitsTestStep(ya_test_runner.exec.step.Python):
	def __init__(self, case: PermitsTestCase):
		self._case = case

	def to_str(self) -> str:
		return '<permits semaphore test>'

	async def run(self, previous_results: list[typing.Any]) -> ya_test_runner.test.Result:
		port = self._case.manager_service.meta['port']
		manager_uri = f'http://localhost:{port}'
		logger = local_ctx.shared.logger

		N = 1
		num_permits = N * 2  # minimum enforced by manager is 2

		original_permits = await self._get_permits(manager_uri)
		genvm_ids: list[str] = []
		bg_tasks: list[asyncio.Task] = []

		shut_downer = asyncio.Event()

		try:
			# Set permits to N * 2
			await self._set_permits(manager_uri, num_permits)
			logger.debug('permits set', permits=num_permits)

			# Start N*2 background genvms (each sync, 1 permit)
			tmp_dir = local_ctx.shared.artifacts_dir / 'permits_test'
			tmp_dir.mkdir(parents=True, exist_ok=True)

			code = BUSY_CONTRACT.read_bytes()

			for i in range(num_permits):
				sock_path = f'/tmp/genvm-test-permits-{i}.sock'
				storage_path = tmp_dir / f'storage_{i}.pickle'

				# Create empty storage
				with open(storage_path, 'wb') as f:
					pickle.dump(MockStorage(), f)

				host = MockHost(
					path=sock_path,
					storage_path_pre=storage_path,
					storage_path_post=storage_path,
					balances={},
					running_address=Address('0x' + '00' * 20),
				)

				ctx = _TestContext(logger)
				task = asyncio.create_task(
					self._run_background_genvm(
						host, manager_uri, ctx, code, sock_path, genvm_ids, shut_downer
					)
				)
				bg_tasks.append(task)

			# Wait for permits to be consumed
			for _ in range(30):
				status = await self._get_status(manager_uri)
				current = status['permits']['current']
				logger.trace('polling permits', current=current)
				if current == 0:
					break
				await asyncio.sleep(0.5)
			else:
				return ya_test_runner.test.Result(
					passed=False,
					context={'reason': 'permits were not consumed within timeout'},
					elapsed_seconds=0,
				)

			logger.debug('all permits consumed, testing blocking behavior')

			# Try to start another genvm — should block on semaphore
			blocked = await self._assert_blocks(manager_uri, code)
			if not blocked:
				return ya_test_runner.test.Result(
					passed=False,
					context={'reason': 'POST /genvm/run did not block when permits exhausted'},
					elapsed_seconds=0,
				)

			logger.info('permits test passed')
			return ya_test_runner.test.Result(passed=True, context={}, elapsed_seconds=0)

		finally:
			shut_downer.set()
			# Shut down running genvms
			for gid in genvm_ids:
				try:
					async with aiohttp.request(
						'DELETE',
						f'{manager_uri}/genvm/{gid}',
						timeout=aiohttp.ClientTimeout(total=10),
					):
						pass
				except Exception as e:
					logger.warning('failed to delete genvm', genvm_id=gid, error=e)
					pass

			for task in bg_tasks:
				try:
					await task
				except (asyncio.CancelledError, Exception):
					pass

			# Restore permits
			await self._set_permits(manager_uri, original_permits)
			logger.debug('permits restored', permits=original_permits)

	async def _run_background_genvm(
		self,
		host: MockHost,
		manager_uri: str,
		ctx: _TestContext,
		code: bytes,
		sock_path: str,
		genvm_ids: list[str],
		shut_downer: asyncio.Event | None = None,
	) -> None:
		with host as mock_host:
			# We wrap run_genvm but intercept the genvm_id.
			# run_genvm blocks until completion; we rely on task cancellation to stop it.
			try:
				await base_host.run_genvm(
					mock_host,
					manager_uri=manager_uri,
					ctx=ctx,
					is_sync=True,
					capture_output=True,
					message=FAKE_MESSAGE,
					host_data='{"node_address":"test","tx_id":"test"}',
					host='unix://' + sock_path,
					code=code,
					calldata=gvm_calldata.encode({}),
					timeout=300,
					extra_args=['--debug-mode'],
					request_extra={'no_modules': True},
					shutdown_early=shut_downer,
				)
			except Exception:
				pass  # expected when cancelled or contract crashes

	async def _assert_blocks(self, manager_uri: str, code: bytes) -> bool:
		"""Make a raw POST /genvm/run and assert it blocks (times out)."""
		data = gvm_calldata.encode(
			{
				'major': 0,
				'message': FAKE_MESSAGE,
				'is_sync': True,
				'capture_output': True,
				'host_data': '{"node_address":"test","tx_id":"test"}',
				'max_execution_minutes': 20,
				'timestamp': '2024-11-26T06:42:42.424242Z',
				'host': 'unix:///tmp/genvm-test-permits-block.sock',
				'extra_args': [],
				'code': code,
				'calldata': gvm_calldata.encode({}),
				'data_fees_limit': 10_000_000,
				'storage_page_cost': 1,
				'receipt_word_cost': 1,
				'no_modules': True,
			}
		)

		try:
			async with asyncio.timeout(5):
				async with aiohttp.request(
					'POST',
					f'{manager_uri}/genvm/run',
					data=data,
					timeout=aiohttp.ClientTimeout(total=10),
				) as resp:
					# If we get here, the request didn't block
					body = await resp.json()
					# Clean up the accidentally started genvm
					if 'id' in body:
						try:
							async with aiohttp.request(
								'DELETE',
								f'{manager_uri}/genvm/{body["id"]}',
								timeout=aiohttp.ClientTimeout(total=5),
							):
								pass
						except Exception:
							pass
					return False
		except (asyncio.TimeoutError, TimeoutError):
			return True

	async def _get_permits(self, manager_uri: str) -> int:
		async with aiohttp.request('GET', f'{manager_uri}/permits') as resp:
			data = await resp.json()
			return data['permits']

	async def _set_permits(self, manager_uri: str, permits: int) -> None:
		async with aiohttp.request(
			'POST',
			f'{manager_uri}/permits',
			json={'permits': permits},
		) as resp:
			if resp.status != 200:
				raise RuntimeError(f'set_permits failed: {resp.status}')

	async def _get_status(self, manager_uri: str) -> dict:
		async with aiohttp.request('GET', f'{manager_uri}/status') as resp:
			return await resp.json()


def collect(
	ctx: ya_test_runner.stage.collection.Context,
	*,
	manager_service: ya_test_runner.stage.collection.Service,
) -> None:
	desc = ya_test_runner.test.Description(
		name='tests/cases/stable/permits',
		needed_services=frozenset({manager_service}),
		tags=frozenset({'stable', 'permits'}),
		console_pool=True,
	)
	ctx.add_case(PermitsTestCase(description=desc, manager_service=manager_service))


local_ctx.plugins['permits_test'] = collect
