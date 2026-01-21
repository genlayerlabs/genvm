import asyncio
import re
import sys
import typing
from pathlib import Path

import ya_test_runner.exec.service

import ya_test_runner

local_ctx = ya_test_runner.stage.configuration.current_context()


class DockerBuildError(Exception):
	pass


async def docker_build(
	*,
	context_dir: Path,
	dockerfile: Path | None = None,
	tag: str | None = None,
	build_args: dict[str, str] | None = None,
	log_context: dict | None = None,
) -> str:
	"""
	Build a Docker image and return its sha256 digest.

	Args:
		context_dir: The directory to use as the build context
		dockerfile: Optional path to the Dockerfile
		tag: Optional tag for the built image
		build_args: Optional build arguments to pass to docker build

	Returns:
		The sha256 digest of the built image (e.g., "sha256:abc123...")

	Raises:
		DockerBuildError: If the build fails or sha256 cannot be parsed
	"""
	args: list[str] = [
		'docker',
		'build',
		'--progress=plain',
	]

	if dockerfile is not None:
		args.extend(['-f', str(dockerfile)])

	if tag is not None:
		args.extend(['-t', tag])

	if build_args:
		for key, value in build_args.items():
			args.extend(['--build-arg', f'{key}={value}'])

	args.append(str(context_dir))

	process = await asyncio.subprocess.create_subprocess_exec(
		*args,
		cwd=context_dir,
		stdout=asyncio.subprocess.PIPE,
		stderr=asyncio.subprocess.PIPE,
	)

	local_ctx.shared.logger.debug(
		'docker build started', context_dir=str(context_dir), args=args
	)

	stdout, stderr = await process.communicate()
	combined_output = stdout.decode('utf-8') + stderr.decode('utf-8')

	if log_context is not None:
		log_context['docker_build_output'] = combined_output

	if process.returncode != 0:
		local_ctx.shared.logger.error(
			'Docker build failed',
			context_dir=str(context_dir),
			args=args,
			exit_code=process.returncode,
			combined_output=combined_output,
		)
		raise DockerBuildError(f'Docker build failed with exit code {process.returncode}')

	sha256_matches = re.findall(
		r'sha256:([a-f0-9]{64})', combined_output, flags=re.IGNORECASE
	)

	if log_context is not None:
		log_context['docker_build_sha256_matches'] = sha256_matches

	if not sha256_matches:
		print(combined_output, file=sys.stderr)
		raise DockerBuildError('Could not find sha256 in docker build output')

	return f'sha256:{sha256_matches[-1]}'


class WebdriverHandle(ya_test_runner.exec.service.Handle):
	def __init__(self, container_id: str, port: int):
		self._container_id = container_id
		self._port = port

	@property
	def port(self) -> int:
		return self._port

	async def healthy(self) -> bool:
		try:
			process = await asyncio.subprocess.create_subprocess_exec(
				'docker',
				'inspect',
				'--format',
				'{{.State.Health.Status}}',
				self._container_id,
				stdout=asyncio.subprocess.PIPE,
				stderr=asyncio.subprocess.DEVNULL,
			)
			stdout, _ = await process.communicate()
			if process.returncode != 0:
				return False
			status = stdout.decode('utf-8').strip()
			return status == 'healthy'
		except Exception:
			return False

	async def interrupt(self) -> None:
		process = await asyncio.subprocess.create_subprocess_exec(
			'docker',
			'stop',
			self._container_id,
			stdout=asyncio.subprocess.DEVNULL,
			stderr=asyncio.subprocess.DEVNULL,
		)
		await process.wait()


class WebdriverService(ya_test_runner.exec.service.Service):
	def __init__(self, *, context_dir: Path, port: int = 4444):
		self._context_dir = context_dir
		self._port = port

	async def start(self) -> WebdriverHandle:
		# Build the image
		build_data = {}
		image_sha = await docker_build(
			context_dir=self._context_dir, log_context=build_data
		)

		# Start the container
		process = await asyncio.subprocess.create_subprocess_exec(
			'docker',
			'run',
			'--rm',
			'-d',
			'-p',
			f'{self._port}:4444',
			image_sha,
			stdout=asyncio.subprocess.PIPE,
			stderr=asyncio.subprocess.PIPE,
		)

		stdout, stderr = await process.communicate()

		if process.returncode != 0:
			local_ctx.shared.logger.warning(
				'Failed to start webdriver container',
				image_sha=image_sha,
				**build_data,
			)
			raise RuntimeError(
				f'Failed to start webdriver container: {stderr.decode("utf-8")}'
			)

		container_id = stdout.decode('utf-8').strip()

		handle = WebdriverHandle(container_id, self._port)

		# Wait for the container to become healthy
		for _ in range(30):
			if await handle.healthy():
				return handle
			await asyncio.sleep(1)

		# Cleanup if health check fails
		await handle.interrupt()
		raise RuntimeError('Webdriver container failed to become healthy')


class ManagerHandle(ya_test_runner.exec.service.Handle):
	def __init__(
		self,
		port: int,
		process: asyncio.subprocess.Process | None,
		log_file: typing.IO[str] | None,
	):
		self._port = port
		self._process = process
		self._log_file = log_file

	@property
	def port(self) -> int:
		return self._port

	@property
	def uri(self) -> str:
		return f'http://localhost:{self._port}'

	async def healthy(self) -> bool:
		import aiohttp

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
			self._process.terminate()
			try:
				await asyncio.wait_for(self._process.wait(), timeout=5)
			except asyncio.TimeoutError:
				self._process.kill()
				await self._process.wait()
		if self._log_file is not None:
			self._log_file.close()


class ManagerService(ya_test_runner.exec.service.Service):
	def __init__(
		self,
		*,
		bin_path: Path,
		port: int = 3999,
		reuse_existing: bool = False,
		reroute_to: str = 'vTEST',
		log_path: Path | None = None,
	):
		self._bin_path = bin_path
		self._port = port
		self._reuse_existing = reuse_existing
		self._reroute_to = reroute_to
		self._log_path = log_path

	@property
	def port(self) -> int:
		return self._port

	async def start(self) -> ManagerHandle:
		# Check if we should reuse existing manager
		if self._reuse_existing:
			handle = ManagerHandle(self._port, process=None, log_file=None)
			if await handle.healthy():
				return handle

		# Start new manager process
		log_file = None
		if self._log_path is not None:
			log_file = open(self._log_path, 'w')

		process = await asyncio.subprocess.create_subprocess_exec(
			str(self._bin_path),
			'manager',
			'--port',
			str(self._port),
			'--reroute-to',
			self._reroute_to,
			'--die-with-parent',
			stdin=asyncio.subprocess.DEVNULL,
			stdout=log_file if log_file else asyncio.subprocess.DEVNULL,
			stderr=log_file if log_file else asyncio.subprocess.DEVNULL,
		)

		handle = ManagerHandle(self._port, process, log_file)

		# Wait for the manager to become healthy
		for _ in range(30):
			if await handle.healthy():
				return handle
			await asyncio.sleep(0.5)

		# Cleanup if health check fails
		await handle.interrupt()
		raise RuntimeError('Manager failed to become healthy')


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
