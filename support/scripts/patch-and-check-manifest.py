#!/usr/bin/env python3

import argparse
from pathlib import Path

from ruamel.yaml import YAML

parser = argparse.ArgumentParser()
parser.add_argument('--tag', required=True, help='current tag')
parser.add_argument('file', help='file to process')

args = parser.parse_args()

yaml = YAML(typ='safe')
doc = yaml.load(Path(args.file).read_text())

executor_versions = doc['executor_versions']

import re

ver_regex = re.compile(r'v(\d+)\.(\d+)\.(\d+)')


def fetch_version_tuple(version_str: str) -> tuple[int, int, int]:
	match = ver_regex.fullmatch(version_str)
	if not match:
		raise ValueError(f'Invalid version string: {version_str}')
	return (int(match.group(1)), int(match.group(2)), int(match.group(3)))


if args.tag not in executor_versions:
	(major, minor, patch) = fetch_version_tuple(args.tag)
	if patch == 0:
		raise ValueError(f'Cannot infer previous version for tag {args.tag}')
	previous_version = f'v{major}.{minor}.{patch - 1}'
	executor_versions[args.tag] = executor_versions[previous_version]

x = list(executor_versions.keys())
x.sort()

for key in x:
	major, minor, patch = fetch_version_tuple(key)
	next_version = f'v{major}.{minor}.{patch + 1}'
	if next_version in executor_versions:
		del executor_versions[key]

import io

res = io.StringIO()
yaml.dump(doc, res)

Path(args.file).write_text(res.getvalue())
