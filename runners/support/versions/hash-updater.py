#!/usr/bin/env python3
"""Automatically fill in runner hashes in ``current.nix``.

Runner archives are content-addressed. A runner's hash also depends on the
*resolved uids* of its dependencies, so the hashes must be discovered in
dependency order: a composite runner (e.g. ``cpython`` -> ``softfloat``, or the
``py-genlayer`` wrappers -> ``cpython``) only hashes correctly once every
dependency already carries its real hash.

This script drives that to a fixed point: it builds ``#debug-runners`` with
``--keep-going``, scrapes every ``hash mismatch ... got: sha256-...`` Nix
reports, writes the ``got`` values back into ``current.nix``, and repeats until
a build produces no more mismatches.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CURRENT_NIX = HERE / 'current.nix'
REPO_ROOT = HERE.parents[2]

# Prefixes attached to attr names to form runner ids (see ``__prefix`` in
# current.nix). Longest first so stripping is unambiguous.
PREFIXES = ('py-lib-', 'models-')

# A hash mismatch spans three consecutive lines:
#     error: hash mismatch in fixed-output derivation '...genvm_runner_<id>_<uid>.drv':
#              specified: sha256-AAAA...
#                 got:    sha256-XXXX...
# The uid is a 52-char hash (all-zeros when the hash is still null).
DRV_RE = re.compile(r'hash mismatch.*genvm_runner_(?P<id>.+?)_[0-9a-z]{52}\.drv')
GOT_RE = re.compile(r'got:\s*(?P<hash>sha256-[A-Za-z0-9+/=]+)')

# current.nix's checkHashes enforces that a runner with a null (or transitively
# null) dependency must itself be null, throwing `set <id> hash to null` for each
# offender. We honor that by nulling those attrs so evaluation can proceed; the
# build loop then refills them bottom-up.
NULL_RE = re.compile(r'set (?P<id>\S+) hash to null')

BUILD_CMD = [
	'nix',
	'build',
	'--keep-going',
	'-L',
	'--no-link',
	f'git+file:{REPO_ROOT}#debug-runners',
]


def id_to_attr(runner_id: str) -> str:
	"""Map a runner id (``py-lib-cloudpickle``) to its attr name in the file."""
	for prefix in PREFIXES:
		if runner_id.startswith(prefix):
			return runner_id[len(prefix) :]
	return runner_id


def parse_mismatches(output: str) -> dict[str, str]:
	"""Return ``{attr_name: got_hash}`` for every reported hash mismatch.

	Walks the log attaching each ``got:`` line to the most recent
	``hash mismatch ... .drv`` line, so the two are never paired across
	unrelated derivations even if builder output interleaves between them.
	"""
	found: dict[str, str] = {}
	pending: str | None = None
	for line in output.splitlines():
		drv = DRV_RE.search(line)
		if drv:
			pending = id_to_attr(drv.group('id'))
			continue
		if pending is not None:
			got = GOT_RE.search(line)
			if got:
				found[pending] = got.group('hash')
				pending = None
	return found


def parse_null_requests(output: str) -> set[str]:
	"""Return the attrs current.nix demands be nulled (dependents of a null hash)."""
	return {id_to_attr(m.group('id')) for m in NULL_RE.finditer(output)}


def map_attr_to_hash_line(lines: list[str]) -> dict[str, int]:
	"""Map each runner attr to the index of its ``hash = ...;`` line.

	Only the ``src`` block is scanned; parsing stops once the helper functions
	(which also contain ``hash = ...`` expressions) begin.
	"""
	attr_open = re.compile(r'^\s*([A-Za-z0-9_-]+)\s*=\s*\{')
	hash_line = re.compile(r'^\s*hash\s*=')
	result: dict[str, int] = {}
	current: str | None = None
	for idx, line in enumerate(lines):
		# End of the `src = rec { ... }` data block, start of helper logic.
		if line.startswith('\thashHasSpecial'):
			break
		m = attr_open.match(line)
		if m:
			current = m.group(1)
			continue
		if current is not None and hash_line.match(line) and current not in result:
			result[current] = idx
	return result


def set_hash(lines: list[str], line_idx: int, value: str) -> None:
	"""Rewrite the ``hash = ...;`` at ``line_idx`` preserving indentation."""
	line = lines[line_idx]
	indent = line[: len(line) - len(line.lstrip())]
	rendered = 'null' if value is None else f'"{value}"'
	lines[line_idx] = f'{indent}hash = {rendered};\n'


def run_build() -> tuple[int, str]:
	proc = subprocess.run(
		BUILD_CMD,
		cwd=REPO_ROOT,
		stdout=subprocess.PIPE,
		stderr=subprocess.STDOUT,
		text=True,
	)
	return proc.returncode, proc.stdout


def main() -> int:
	ap = argparse.ArgumentParser(description=__doc__)
	ap.add_argument(
		'--reset',
		action='store_true',
		help='set every runner hash to null before building (full recompute)',
	)
	ap.add_argument(
		'--dry-run',
		action='store_true',
		help='report discovered hashes without writing them back',
	)
	ap.add_argument(
		'--max-iters',
		type=int,
		default=10,
		help='give up after this many build passes (default: 10)',
	)
	args = ap.parse_args()

	lines = CURRENT_NIX.read_text().splitlines(keepends=True)
	attr_lines = map_attr_to_hash_line(lines)

	if args.reset:
		for idx in attr_lines.values():
			set_hash(lines, idx, None)
		if not args.dry_run:
			CURRENT_NIX.write_text(''.join(lines))
		print(f'reset {len(attr_lines)} hashes to null')

	last_actions: dict[str, str | None] = {}
	for iteration in range(1, args.max_iters + 1):
		print(f'== build pass {iteration} ==', flush=True)
		code, output = run_build()

		# null-propagation throws happen at eval time, before anything builds, so
		# they never coincide with mismatches; resolve them first.
		null_reqs = parse_null_requests(output)
		mismatches = parse_mismatches(output)

		if null_reqs:
			actions: dict[str, str | None] = dict.fromkeys(null_reqs, None)
		elif mismatches:
			actions = dict(mismatches)
		elif code == 0:
			print('build succeeded, all hashes are correct')
			return 0
		else:
			# Build failed for some non-hash reason; surface the tail.
			print('build failed with no hash mismatches:', file=sys.stderr)
			print('\n'.join(output.splitlines()[-30:]), file=sys.stderr)
			return 1

		unknown = sorted(set(actions) - set(attr_lines))
		if unknown:
			print(f"error: no attr in current.nix for: {', '.join(unknown)}", file=sys.stderr)
			return 1

		for attr, h in sorted(actions.items()):
			print(f'  {attr} -> {h if h is not None else "null"}')

		if actions == last_actions:
			print('error: same actions as last pass, not converging', file=sys.stderr)
			return 1
		last_actions = actions

		if args.dry_run:
			print('dry-run: not writing')
			return 0

		for attr, h in actions.items():
			set_hash(lines, attr_lines[attr], h)
		CURRENT_NIX.write_text(''.join(lines))

	print(f'error: did not converge in {args.max_iters} passes', file=sys.stderr)
	return 1


if __name__ == '__main__':
	sys.exit(main())
