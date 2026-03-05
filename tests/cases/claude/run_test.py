#!/usr/bin/env python3

"""
Runs a specific claude integration test with --no-manager.
Assumes manager is already running externally.

Usage:
	./run_test.py <test-dir>
	./run_test.py example

This will run:
	ya-test-runner --filter-tag integration --filter-name 'tests/cases/stable/claude/<test-dir>' run --no-manager
"""

import subprocess
import sys
import os
from pathlib import Path

root_dir = Path(__file__).resolve()

while (
	root_dir.parent != root_dir and not root_dir.joinpath('.genvm-monorepo-root').exists()
):
	root_dir = root_dir.parent

binary = root_dir.joinpath('.direnv', 'ya-test-runner', 'bin', 'ya-test-runner')
if not binary.exists():
	print(f'Error: could not find ya-test-runner binary at {binary}', file=sys.stderr)
	sys.exit(1)


def main():
	if len(sys.argv) < 2:
		print(f'Usage: {sys.argv[0]} <test-dir> [extra-args...]', file=sys.stderr)
		print(f'Example: {sys.argv[0]} example', file=sys.stderr)
		sys.exit(1)

	test_dir = sys.argv[1]
	extra_args = sys.argv[2:]

	test_name = f'tests/cases/claude/{test_dir}'

	cmd = [
		binary,
		'--filter-tag',
		'integration',
		'--filter-name',
		test_name,
		'run',
		'--no-manager',
		*extra_args,
	]

	os.chdir(root_dir)
	os.execvp(cmd[0], cmd)


if __name__ == '__main__':
	main()
