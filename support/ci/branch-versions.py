#!/usr/bin/env python3

"""
Read the active version trains from .genvm-monorepo-root.

The `active-versions` key holds every release train that is currently
live (e.g. ["0.3", "0.6"]). The branch-model workflows derive branch
names from it:
	- version branch: v<X>.x   (release-ready)
	- dev branch:     v<X>-dev (integration)

Usage:
	branch-versions.py list      # all active versions, ascending, one per line
	branch-versions.py latest    # the highest active version
"""

import os
import json
import sys
import argparse
from pathlib import Path

# MONOREPO_ROOT lets a caller point at a specific .genvm-monorepo-root
# (e.g. one extracted from origin/main) instead of the working tree's.
_default = Path(__file__).resolve().parents[2] / '.genvm-monorepo-root'
_conf = json.loads(Path(os.environ.get('MONOREPO_ROOT', _default)).read_bytes())
_versions = _conf.get('active-versions', [])


def _key(v: str):
	return tuple(int(x) for x in v.split('.'))


_versions = sorted(_versions, key=_key)


def main() -> int:
	parser = argparse.ArgumentParser(
		description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
	)
	parser.add_argument('cmd', nargs='?', choices=['list', 'latest'], default='list')
	args = parser.parse_args()

	if args.cmd == 'list':
		print('\n'.join(_versions))
		return 0
	if not _versions:
		print('no active versions configured in .genvm-monorepo-root', file=sys.stderr)
		return 1
	print(_versions[-1])
	return 0


if __name__ == '__main__':
	sys.exit(main())
