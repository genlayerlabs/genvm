import os
import re
import sys
from pathlib import Path
import copy

import ya_test_runner.exec.service

import ya_test_runner

local_ctx = ya_test_runner.stage.configuration.current_context()


class DockerBuildError(Exception):
	pass


class DockerRunError(Exception):
	pass


DEFAULT_ENV = {
	k: v
	for k, v in os.environ.items()
	if ya_test_runner.util.environ.DEFAULT_FILTER(k, v)
}
DEFAULT_ENV.pop('LD_LIBRARY_PATH', None)


async def build(
	*,
	context_dir: Path,
	dockerfile: Path | None = None,
	tag: str | None = None,
	build_args: dict[str, str] | None = None,
	log_context: dict | None = None,
	env: dict[str, str] | None = None,
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

	if env is None:
		env = copy.deepcopy(DEFAULT_ENV)

	cmd = ya_test_runner.exec.command.Command(args=args, cwd=context_dir, env=env)

	res = await cmd.run(
		ctx=local_ctx.shared, mode=ya_test_runner.exec.command.RunMode.SILENT
	)

	combined_output = res.stdout + res.stderr

	if log_context is not None:
		log_context['docker_build_stdout'] = res.stdout
		log_context['docker_build_stderr'] = res.stderr

	if res.exit_code != 0:
		local_ctx.shared.logger.error(
			'Docker build failed',
			context_dir=str(context_dir),
			args=args,
			env=env,
			exit_code=res.exit_code,
			combined_output=combined_output,
		)
		raise DockerBuildError(f'Docker build failed with exit code {res.exit_code}')

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
		self._stop_command = ['docker', 'stop', container_id]
		local_ctx.shared.watchdog.add_command(self._stop_command)

	async def get_logs(self, tail: int = 100) -> str:
		"""Get recent logs from the container."""
		cmd = ya_test_runner.exec.command.Command(
			args=['docker', 'logs', '--tail', str(tail), self._container_id],
			cwd=local_ctx.shared.root_dir,
			env=DEFAULT_ENV,
		)
		res = await cmd.run(
			ctx=local_ctx.shared, mode=ya_test_runner.exec.command.RunMode.SILENT
		)
		return res.stdout + res.stderr

	@staticmethod
	async def run(
		args: list[str | Path],
		*,
		cwd: Path,
		env: dict[str, str] | None = None,
		log_context: dict | None = None,
	) -> 'ContainerHandle':
		if env is None:
			env = copy.deepcopy(DEFAULT_ENV)
		cmd = ya_test_runner.exec.command.Command(
			args=[
				'docker',
				'run',
				'--rm',
				'-d',
				*args,
			],
			cwd=cwd,
			env=env,
		)

		res = await cmd.run(
			ctx=local_ctx.shared, mode=ya_test_runner.exec.command.RunMode.SILENT
		)

		combined_output = res.stdout + res.stderr

		if log_context is not None:
			log_context['docker_run_stdout'] = res.stdout
			log_context['docker_run_stderr'] = res.stderr

		if res.exit_code != 0:
			local_ctx.shared.logger.error(
				'Docker run failed',
				cwd=str(cwd),
				args=args,
				env=env,
				exit_code=res.exit_code,
				combined_output=combined_output,
			)
			raise DockerRunError(f'Docker run failed with exit code {res.exit_code}')
		container_id = res.stdout.strip()
		return ContainerHandle(container_id=container_id)

	async def healthy(self) -> bool:
		try:
			cmd = ya_test_runner.exec.command.Command(
				args=[
					'docker',
					'inspect',
					'--format',
					'{{.State.Health.Status}}',
					self._container_id,
				],
				cwd=local_ctx.shared.root_dir,
				env=DEFAULT_ENV,
			)
			res = await cmd.run(
				ctx=local_ctx.shared, mode=ya_test_runner.exec.command.RunMode.SILENT
			)
			if res.exit_code != 0:
				logs = await self.get_logs()
				local_ctx.shared.logger.warning(
					'Docker health check failed',
					container_id=self._container_id,
					returncode=res.exit_code,
					logs=logs,
				)
				return False
			status = res.stdout.strip()
			if status != 'healthy':
				logs = await self.get_logs()
				local_ctx.shared.logger.warning(
					'Container not healthy',
					container_id=self._container_id,
					status=status,
					logs=logs,
				)
			return status == 'healthy'
		except Exception as e:
			local_ctx.shared.logger.error(
				'Failed to check container health',
				container_id=self._container_id,
				error=e,
			)
			return False

	async def interrupt(self) -> None:
		cmd = ya_test_runner.exec.command.Command(
			args=self._stop_command,
			cwd=local_ctx.shared.root_dir,
			env=DEFAULT_ENV,
		)
		await cmd.run(ctx=local_ctx.shared, mode=ya_test_runner.exec.command.RunMode.SILENT)

		local_ctx.shared.watchdog.remove_command(self._stop_command)
