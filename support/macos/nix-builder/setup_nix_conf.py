#!/usr/bin/env python3
import platform
import sys
from pathlib import Path

machine = platform.machine()
native = {'aarch64': 'aarch64-linux', 'x86_64': 'x86_64-linux'}.get(machine)
if native is None:
	print(f'unsupported machine: {machine}', file=sys.stderr)
	sys.exit(1)
foreign = 'x86_64-linux' if native == 'aarch64-linux' else 'aarch64-linux'

Path('/etc/nix').mkdir(parents=True, exist_ok=True)
Path('/etc/nix/nix.conf').write_text(
	f"""\
experimental-features = nix-command flakes
sandbox = true
sandbox-fallback = false
build-users-group =
system = {native}
extra-platforms = {foreign}
"""
)
