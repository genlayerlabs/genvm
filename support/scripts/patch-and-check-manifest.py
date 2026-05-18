#!/usr/bin/env python3

import argparse
from pathlib import Path

from ruamel.yaml import YAML

parser = argparse.ArgumentParser()
parser.add_argument('--tag', required=True, help='current tag')
parser.add_argument('file', help='file to process')

args = parser.parse_args()

yaml = YAML(typ='rt')
doc = yaml.load(Path(args.file).read_text())

executor_versions = doc['executor_versions']

import re

ver_regex = re.compile(r'v(\d+)\.(\d+)\.(\d+)(?:-.*)?')


def fetch_version_tuple(version_str: str) -> tuple[int, int, int]:
	match = ver_regex.fullmatch(version_str)
	if not match:
		raise ValueError(f'Invalid version string: {version_str}')
	return (int(match.group(1)), int(match.group(2)), int(match.group(3)))


if args.tag not in executor_versions:
	(major, minor, patch) = fetch_version_tuple(args.tag)
	patched = False
	for i in range(patch):
		prev_patch = patch - 1 - i
		previous_version = f'v{major}.{minor}.{prev_patch}'
		if previous_version in executor_versions:
			if not patched:
				executor_versions[args.tag] = executor_versions[previous_version]
				patched = True
			del executor_versions[previous_version]
	if not patched:
		raise ValueError(f'Could not find any previous version for tag {args.tag}')

x = list(executor_versions.keys())
x.sort()

all_tuples = {fetch_version_tuple(k) for k in x}

for key in x:
	major, minor, patch = fetch_version_tuple(key)
	if (major, minor, patch + 1) in all_tuples:
		del executor_versions[key]

import io

res = io.StringIO()
yaml.dump(doc, res)

yaml_val = res.getvalue()

Path(args.file).write_text(yaml_val)
