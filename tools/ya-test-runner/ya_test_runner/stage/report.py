from ya_test_runner import SharedContext

import typing
from ya_test_runner import SharedContext

from .execution import Env as ExecutionEnv


def run(shared: SharedContext, exec_env: ExecutionEnv) -> bool:
	passed = len(exec_env.failed) == 0
	shared.printer.put(
		'Test execution summary',
		success_count=exec_env.success_count,
		failed_count=len(exec_env.failed),
		failed=exec_env.failed,
		passed=passed,
	)
	return passed
