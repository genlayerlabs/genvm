#!/usr/bin/env python3

import argparse
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--tag', required=True, help='current tag')
parser.add_argument('file', help='file to process')

args = parser.parse_args()


def process_one(p: Path):
	if p.is_dir():
		for child in p.iterdir():
			process_one(child)
		return

	if not p.is_file() or p.is_symlink():
		return

	if not p.name.endswith(('.yml', '.yaml')):
		return

	text = p.read_text().splitlines()

	schema_prefix = '# yaml-language-server: $schema='
	bad_prefix = schema_prefix + '../'

	if len(text) > 0 and text[0].startswith(bad_prefix):
		text.pop(0)

	if len(text) > 0 and text[0].startswith(schema_prefix):
		text[0] = text[0].replace('refs/heads/main', f'refs/tags/{args.tag}')

	Path(p).write_text('\n'.join(text) + '\n')


process_one(Path(args.file))
