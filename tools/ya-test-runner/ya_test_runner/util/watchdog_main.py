#!/usr/bin/env python3
"""
Watchdog process for ya-test-runner.

Started by the test runner as a child process. Monitors whether its parent
(the test runner) is still alive by checking if it has been reparented.

If reparented (parent died):
1. Kills all processes in the original process group
2. Executes cleanup commands registered by the test runner (e.g. docker stop)
3. Exits

Communication protocol (via stdin, JSON lines):
- {"action": "add", "command": ["docker", "stop", "container_id"]}
- {"action": "remove", "command": ["docker", "stop", "container_id"]}
"""

import json
import os
import signal
import subprocess
import select
import sys


def main():
	original_ppid = os.getppid()
	original_pgrp = os.getpgrp()

	commands: list[list[str]] = []
	buffer = ''
	stdin_fd = sys.stdin.fileno()

	while True:
		if os.getppid() != original_ppid:
			break

		try:
			ready, _, _ = select.select([stdin_fd], [], [], 1.0)
		except (ValueError, OSError):
			return

		if ready:
			try:
				data = os.read(stdin_fd, 4096)
			except OSError:
				return
			if not data:
				# stdin closed - parent finished normally
				return

			buffer += data.decode('utf-8', errors='replace')
			while '\n' in buffer:
				line, buffer = buffer.split('\n', 1)
				line = line.strip()
				if not line:
					continue
				try:
					msg = json.loads(line)
					action = msg.get('action')
					cmd = msg.get('command')
					if action == 'add' and isinstance(cmd, list):
						commands.append(cmd)
					elif action == 'remove' and isinstance(cmd, list):
						try:
							commands.remove(cmd)
						except ValueError:
							pass
				except (json.JSONDecodeError, KeyError, TypeError):
					pass

	# Reparented - parent died. Clean up.

	# 1. Move to own process group so we survive the group kill
	try:
		os.setpgrp()
	except OSError:
		pass

	# Kill all processes in the original process group
	try:
		os.killpg(original_pgrp, signal.SIGKILL)
	except (ProcessLookupError, PermissionError, OSError):
		pass

	# 2. Execute cleanup commands
	for cmd in commands:
		try:
			subprocess.run(cmd, timeout=30, capture_output=True)
		except Exception:
			pass

	sys.exit(0)


if __name__ == '__main__':
	main()
