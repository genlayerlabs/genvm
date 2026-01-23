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


async def build(
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


class ContainerHandle(ya_test_runner.exec.service.Handle):
	def __init__(self, container_id: str):
		self._container_id = container_id

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
		except Exception as e:
			local_ctx.shared.logger.error(
				'Failed to check container health',
				container_id=self._container_id,
				error=e,
			)
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
