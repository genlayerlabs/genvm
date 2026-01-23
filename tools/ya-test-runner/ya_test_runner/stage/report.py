from ya_test_runner import SharedContext

import random
import string
import typing
from datetime import datetime

from .execution import Env as ExecutionEnv


def _write_continue_file(shared: SharedContext, failed_tests: list[str]) -> str | None:
	"""Write failed test names to a continue file for later re-runs."""
	if not failed_tests:
		return None

	continue_dir = shared.artifacts_dir / 'continue'
	continue_dir.mkdir(parents=True, exist_ok=True)

	# Generate filename: <date>-<random>
	date_str = datetime.now().strftime('%Y%m%d-%H%M%S')
	random_suffix = ''.join(random.choices(string.ascii_lowercase + string.digits, k=6))
	filename = f'{date_str}-{random_suffix}'
	filepath = continue_dir / filename

	# Write failed test names, one per line
	filepath.write_text('\n'.join(failed_tests) + '\n')

	return str(filepath)


def run(shared: SharedContext, exec_env: ExecutionEnv) -> bool:
	passed = len(exec_env.failed) == 0

	# Write continue file if there are failures
	continue_file = None
	if not passed:
		continue_file = _write_continue_file(shared, exec_env.failed)

	shared.printer.put(
		'Test execution summary',
		success_count=exec_env.success_count,
		failed_count=len(exec_env.failed),
		failed=exec_env.failed,
		passed=passed,
		continue_file=continue_file,
	)
	return passed
