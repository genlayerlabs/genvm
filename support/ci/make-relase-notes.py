#!/usr/bin/env python3
"""Generate categorized release notes from conventional commits."""

import argparse
import subprocess
import sys
import re
import unicodedata
from collections import defaultdict
from dataclasses import dataclass


def strip_trailing_emoji(text: str) -> str:
	"""Drop the trailing gitmoji suffix (and its variation selectors/ZWJ).

	Commit subjects follow ``type(scope): summary <emoji>``; the category
	heading already conveys the type, so the emoji is redundant noise here.
	"""
	chars = list(text)
	while chars:
		c = chars[-1]
		if c.isspace() or c in '\ufe0f\u200d' or unicodedata.category(c) in ('So', 'Sk'):
			chars.pop()
		else:
			break
	return ''.join(chars).rstrip()


NOREPLY_RE = re.compile(
	r'(?:\d+\+)?(?P<handle>[\w.\[\]-]+)@users\.noreply\.github\.com'
)


def format_author(email: str) -> str:
	"""Collapse GitHub noreply addresses to ``@handle``; keep real emails as-is."""
	m = NOREPLY_RE.fullmatch(email)
	return f'@{m.group("handle")}' if m else email


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

	Body lines that look like ``* type: description`` are promoted to top-level
	entries so they get categorised on their own (inheriting the commit's
	author).  When a commit has such promoted lines its subject is treated as a
	squash/PR umbrella title and dropped — the bullets already carry the real
	changes, so keeping the subject would double-count them.
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
		# body lines starting with * may carry their own conventional prefix
		promoted = [
			CommitEntry(cleaned, email)
			for line in lines[1:]
			if COMMIT_RE.match(cleaned := re.sub(r'^\*\s*', '', line))
		]
		if promoted:
			entries.extend(promoted)
		else:
			# no bullets: the subject is the change itself
			entries.append(CommitEntry(lines[0], email))
	return entries


def categorize(entries: list[CommitEntry]) -> dict[str, list[str]]:
	categories: dict[str, list[str]] = defaultdict(list)
	for entry in entries:
		m = COMMIT_RE.match(entry.message)
		if m:
			ctype = m.group('type')
			scope = m.group('scope')
			desc = strip_trailing_emoji(m.group('desc').strip())
			text = f'**{scope}**: {desc}' if scope else desc
			text += f' ({format_author(entry.email)})'
			categories[ctype].append(text)
		else:
			categories['other'].append(
				f'{strip_trailing_emoji(entry.message)} ({format_author(entry.email)})'
			)
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
	parser = argparse.ArgumentParser(
		description='Generate categorized release notes from conventional commits.',
	)
	parser.add_argument(
		'rev_range',
		metavar='rev-range',
		help='git revision range, e.g. v0.2.7..v0.2.8',
	)
	args = parser.parse_args()

	entries = parse_git_log(args.rev_range)
	if not entries:
		print('No commits found.', file=sys.stderr)
		sys.exit(1)

	categories = categorize(entries)
	print(format_notes(categories, args.rev_range))


if __name__ == '__main__':
	main()
