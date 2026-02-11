#!/usr/bin/env python3
"""Generate categorized release notes from conventional commits."""

import subprocess
import sys
import re
from collections import defaultdict
from dataclasses import dataclass

CATEGORY_LABELS = {
	'feat': 'Features',
	'fix': 'Bug Fixes',
	'perf': 'Performance',
	'refactor': 'Refactoring',
	'docs': 'Documentation',
	'test': 'Tests',
	'ci': 'CI',
	'build': 'Build',
	'chore': 'Chores',
}

COMMIT_RE = re.compile(r'^(?P<type>[a-z]+)(?:\((?P<scope>[^)]+)\))?!?:\s*(?P<desc>.+)$')


@dataclass
class CommitEntry:
	message: str
	email: str


def parse_git_log(rev_range: str) -> list[CommitEntry]:
	"""Run git log and return parsed commit entries.

	Each commit's subject line is included directly.  Body lines that look
	like ``* type: description`` are promoted to top-level entries so they
	get categorised on their own (inheriting the commit's author).
	"""
	result = subprocess.run(
		['git', 'log', '--pretty=format:%ae%x01%B%x00', rev_range],
		capture_output=True,
		text=True,
		check=True,
	)
	entries: list[CommitEntry] = []
	for raw_commit in result.stdout.split('\0'):
		raw_commit = raw_commit.strip()
		if not raw_commit:
			continue
		email, _, body = raw_commit.partition('\x01')
		lines = [l.strip() for l in body.strip().splitlines() if l.strip()]
		if not lines:
			continue
		# first line is the subject
		entries.append(CommitEntry(lines[0], email))
		# body lines starting with * may carry their own conventional prefix
		for line in lines[1:]:
			cleaned = re.sub(r'^\*\s*', '', line)
			if COMMIT_RE.match(cleaned):
				entries.append(CommitEntry(cleaned, email))
	return entries


def categorise(entries: list[CommitEntry]) -> dict[str, list[str]]:
	categories: dict[str, list[str]] = defaultdict(list)
	for entry in entries:
		m = COMMIT_RE.match(entry.message)
		if m:
			ctype = m.group('type')
			scope = m.group('scope')
			desc = m.group('desc').strip()
			text = f'**{scope}**: {desc}' if scope else desc
			text += f' ({entry.email})'
			categories[ctype].append(text)
		else:
			categories['other'].append(f'{entry.message} ({entry.email})')
	return categories


REPO_URL = 'https://github.com/genlayerlabs/genvm'


def format_notes(categories: dict[str, list[str]], rev_range: str) -> str:
	parts: list[str] = []
	# ordered by CATEGORY_LABELS first, then anything remaining
	ordered_keys = [k for k in CATEGORY_LABELS if k in categories]
	extra_keys = sorted(k for k in categories if k not in CATEGORY_LABELS)
	for key in ordered_keys + extra_keys:
		items = categories[key]
		# deduplicate while preserving order
		seen: set[str] = set()
		unique: list[str] = []
		for item in items:
			low = item.lower()
			if low not in seen:
				seen.add(low)
				unique.append(item)
		label = CATEGORY_LABELS.get(key, key.title())
		parts.append(f'### {label}\n')
		for item in unique:
			parts.append(f'- {item}')
		parts.append('')
	compare = rev_range.replace('..', '...')
	parts.append(f'**Full Changelog**: {REPO_URL}/compare/{compare}')
	return '\n'.join(parts).strip() + '\n'


def main() -> None:
	if len(sys.argv) < 2:
		print(f'Usage: {sys.argv[0]} <rev-range>', file=sys.stderr)
		print(f'  e.g. {sys.argv[0]} v0.2.7..v0.2.8', file=sys.stderr)
		sys.exit(1)

	rev_range = sys.argv[1]
	entries = parse_git_log(rev_range)
	if not entries:
		print('No commits found.', file=sys.stderr)
		sys.exit(1)

	categories = categorise(entries)
	print(format_notes(categories, rev_range))


if __name__ == '__main__':
	main()
