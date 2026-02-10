import asyncio
import json
import os
import aiohttp
import typing
from pathlib import Path

import ya_test_runner.exec.service

import ya_test_runner

import ya_test_runner_plugins.docker as docker

local_ctx = ya_test_runner.stage.configuration.current_context()


def get_webdriver_port(env: ya_test_runner.stage.configuration.Env) -> int:
	return 4444


def get_manager_port(env: ya_test_runner.stage.configuration.Env) -> int:
	return 3999


async def start_webdriver_service(
	env: ya_test_runner.stage.configuration.Env,
) -> ya_test_runner.exec.service.Handle:
	context_dir = local_ctx.shared.root_dir.joinpath('modules', 'webdriver')

	build_data = {}
	image_sha = await docker.build(context_dir=context_dir, log_context=build_data)

	port = get_webdriver_port(env)

	proc_env = docker.DEFAULT_ENV.copy()
	for k, v in os.environ.items():
		if k.lower().endswith('key') or k.lower().endswith('token'):
			proc_env[k] = v

	return await docker.ContainerHandle.run(
		['-p', f'{port}:4444', image_sha], cwd=local_ctx.shared.root_dir, env=proc_env
	)


_LOG_LEVEL_PRIORITY = {
	'trace': 0,
	'debug': 1,
	'info': 2,
	'warn': 3,
	'warning': 3,
	'error': 4,
}


async def _read_log_pipe(
	stream: asyncio.StreamReader,
	log_file: typing.IO[str],
	logger: ya_test_runner.Formatter,
) -> None:
	while True:
		line_bytes = await stream.readline()
		if not line_bytes:
			break
		line = line_bytes.decode('utf-8', errors='replace').rstrip('\n')
		log_file.write(line + '\n')
		log_file.flush()
		try:
			parsed = json.loads(line)
			level = parsed.get('level', '')
			if _LOG_LEVEL_PRIORITY.get(level, -1) >= _LOG_LEVEL_PRIORITY['info']:
				message = parsed.get('message', line)
				extra = {k: v for k, v in parsed.items() if k not in ('level', 'message')}
				fmt_level = ya_test_runner.Formatter.Level.from_str(level)
				logger.log(fmt_level, f'[manager] {message}', **extra)
		except (json.JSONDecodeError, ValueError):
			logger.warning(f'[manager] {line}')


class ManagerHandle(ya_test_runner.exec.service.Handle):
	def __init__(
		self,
		port: int,
		process: asyncio.subprocess.Process | None,
		log_file: typing.IO[str] | None,
		log_tasks: list[asyncio.Task] | None,
	):
		self._port = port
		self._process = process
		self._log_file = log_file
		self._log_tasks = log_tasks

	@property
	def port(self) -> int:
		return self._port

	@property
	def uri(self) -> str:
		return f'http://localhost:{self._port}'

	async def healthy(self) -> bool:
		try:
			async with aiohttp.ClientSession() as session:
				async with session.get(
					f'{self.uri}/status',
					timeout=aiohttp.ClientTimeout(total=5),
				) as response:
					return response.status == 200
		except Exception:
			return False

	async def interrupt(self) -> None:
		if self._process is not None:
			try:
				self._process.terminate()
			except Exception:
				pass

			try:
				await asyncio.wait_for(self._process.wait(), timeout=5)
			except asyncio.TimeoutError:
				self._process.kill()
				await self._process.wait()
		if self._log_tasks is not None:
			for task in self._log_tasks:
				await task
		if self._log_file is not None:
			self._log_file.close()


class ManagerService(ya_test_runner.exec.service.Service):
	def __init__(
		self,
		*,
		env: ya_test_runner.stage.configuration.Env,
		bin_path: Path,
		reroute_to: str = 'vTEST',
		log_path: Path | None = None,
	):
		self._bin_path = bin_path
		self._reroute_to = reroute_to
		self._log_path = log_path
		self._env = env

	async def start(self) -> ManagerHandle:
		log_file = None
		log_tasks = None
		use_pipe = self._log_path is not None

		if use_pipe:
			log_file = open(self._log_path, 'w')

		port = get_manager_port(self._env)

		process = await asyncio.subprocess.create_subprocess_exec(
			str(self._bin_path),
			'manager',
			'--port',
			str(port),
			'--reroute-to',
			self._reroute_to,
			'--die-with-parent',
			stdin=asyncio.subprocess.DEVNULL,
			stdout=asyncio.subprocess.PIPE if use_pipe else asyncio.subprocess.DEVNULL,
			stderr=asyncio.subprocess.PIPE if use_pipe else asyncio.subprocess.DEVNULL,
			start_new_session=True,
		)

		if use_pipe:
			logger = local_ctx.shared.logger
			log_tasks = [
				asyncio.create_task(_read_log_pipe(process.stdout, log_file, logger)),
				asyncio.create_task(_read_log_pipe(process.stderr, log_file, logger)),
			]

		handle = ManagerHandle(port, process, log_file, log_tasks)

		return handle


class ModulesHandle(ya_test_runner.exec.service.Handle):
	"""Handle for started modules - no cleanup needed as modules stop with manager."""

	def __init__(self, manager_uri: str):
		self._manager_uri = manager_uri

	async def healthy(self) -> bool:
		return True  # Modules are healthy if they started successfully

	async def interrupt(self) -> None:
		pass  # Modules stop when manager stops


class ModulesService(ya_test_runner.exec.service.Service):
	"""
	Service that starts LLM and Web modules on the manager.

	This service depends on ManagerService being started first.
	"""

	def __init__(self, *, manager_uri: str):
		self._manager_uri = manager_uri

	async def start(self) -> ModulesHandle:
		import aiohttp

		async def start_module(name: str) -> None:
			async with aiohttp.request(
				'POST',
				f'{self._manager_uri}/module/start',
				json={'module_type': name, 'config': None},
			) as resp:
				body = await resp.json()
				if resp.status != 200 and body != {'error': 'module_already_running'}:
					raise RuntimeError(f'Starting module {name} failed: {resp.status} {body}')

		# Start both modules
		await asyncio.gather(
			start_module('Llm'),
			start_module('Web'),
		)

		return ModulesHandle(self._manager_uri)
