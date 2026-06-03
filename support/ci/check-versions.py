#!/usr/bin/env python3
import re
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[2]
major_minor = (root / 'support' / 'ci' / 'MAJOR_MINOR').read_text().strip()


def package_version(path):
	section = None
	for line in path.read_text().splitlines():
		s = line.strip()
		m = re.match(r'\[(.+?)\]', s)
		if m:
			section = m.group(1)
			continue
		if section == 'package':
			m = re.match(r'version\s*=\s*"([^"]+)"', s)
			if m:
				return m.group(1)
	return None


files = ['executor/Cargo.toml', 'modules/implementation/Cargo.toml']
exit_code = 0
for f in files:
	version = package_version(root / f)
	if version is None:
		print(f'{f}: could not find [package] version')
		exit_code = 1
		continue
	mm = '.'.join(version.split('.')[:2])
	if mm != major_minor:
		print(
			f'{f}: version {version} (major.minor {mm}) does not match support/ci/MAJOR_MINOR ({major_minor})'
		)
		exit_code = 1

sys.exit(exit_code)
