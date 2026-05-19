#!/usr/bin/env python3
import os
import shutil
import signal
import subprocess
import sys
from pathlib import Path

VOLUME = Path('/var/lib/nix-builder')
VOLUME.mkdir(parents=True, exist_ok=True)

vol_server_key = VOLUME / 'id_ed25519'
vol_server_pub = VOLUME / 'id_ed25519.pub'
vol_host_pub = VOLUME / 'host.pub'

if not vol_server_key.exists():
	subprocess.check_call(
		[
			'ssh-keygen',
			'-t',
			'ed25519',
			'-N',
			'',
			'-f',
			str(vol_server_key),
			'-C',
			'nix-builder',
		]
	)
	os.chmod(vol_server_key, 0o666)
	os.chmod(vol_server_pub, 0o666)

if not vol_host_pub.exists():
	print(
		f"missing {vol_host_pub}: provide the connecting host's public key in the volume",
		file=sys.stderr,
	)
	print('server host public key:')
	print(vol_server_pub.read_text().strip())
	sys.exit(1)

host_key = Path('/etc/ssh/ssh_host_ed25519_key')
authorized = Path('/etc/ssh/authorized_keys')

host_key.write_bytes(vol_server_key.read_bytes())
os.chmod(host_key, 0o600)

authorized.write_bytes(vol_host_pub.read_bytes())
os.chmod(authorized, 0o600)

sshd = shutil.which('sshd')
if sshd is None:
	print('sshd not found on PATH', file=sys.stderr)
	sys.exit(1)

proc = subprocess.Popen([sshd, '-D', '-e', '-f', '/etc/ssh/sshd_config'])


def _forward(signum, _frame):
	proc.send_signal(signal.SIGTERM if signum == signal.SIGINT else signum)


signal.signal(signal.SIGINT, _forward)
signal.signal(signal.SIGTERM, _forward)
signal.signal(signal.SIGHUP, _forward)

proc.wait()
sys.exit(proc.returncode)
