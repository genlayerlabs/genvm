#!/usr/bin/env python3

import subprocess, json, sys, re
from pathlib import Path

proc = subprocess.run(
	['git', 'remote', 'get-url', 'origin'],
	check=True,
	capture_output=True,
	text=True,
)

origin_url = proc.stdout.strip()

proc = subprocess.run(
	['git', 'ls-remote', '--tags', origin_url],
	check=True,
	capture_output=True,
	text=True,
)

commit2tag = {}

bad_id = re.compile(r'[^0-9a-z._\-A-Z]')

for line in proc.stdout.splitlines():
	line = line.strip()
	comm_rev = line.split()
	if len(comm_rev) != 2:
		continue
	commit, rev = comm_rev

	rev = rev.removeprefix('refs/tags/')

	commit2tag[commit] = bad_id.sub('_', rev)

import os

cwd = Path(os.getcwd())

# Ensure tags are available locally (CI may use shallow checkout)
subprocess.run(['git', 'fetch', '--tags'], capture_output=True)

# Collect tag messages (one-line) for descriptions
proc_tags = subprocess.run(
	['git', 'tag', '-l', '-n1'],
	check=True,
	capture_output=True,
	text=True,
)
tag_messages = {}
for line in proc_tags.stdout.splitlines():
	parts = line.split(None, 1)
	if len(parts) == 2:
		tag_messages[parts[0]] = parts[1].strip()
	elif len(parts) == 1:
		tag_messages[parts[0]] = ''

current_commit = subprocess.run(
	['git', 'rev-parse', 'HEAD'],
	check=True,
	capture_output=True,
	text=True,
)

current_commit = current_commit.stdout.strip()

eval_file = Path(__file__).parent.parent.joinpath('docs.nix')

build_config = json.loads(
	Path(__file__).parent.parent.parent.joinpath('flake-config.json').read_text()
)
build_config['head-revision'] = current_commit
commit2tag[current_commit] = build_config['executor-version']

print(commit2tag)
print(build_config)

proc = subprocess.run(
	[
		'nix',
		'eval',
		'--verbose',
		'--impure',
		'--read-only',
		'--show-trace',
		'--json',
		'--file',
		str(eval_file.relative_to(cwd)),
		'--apply',
		'f: f { commitToTagStr = "'
		+ json.dumps(commit2tag).replace('"', '\\"')
		+ '"; build-config-str = "'
		+ json.dumps(build_config).replace('"', '\\"')
		+ '"; }',
	],
	check=True,
	capture_output=True,
	text=True,
)

res = json.loads(proc.stdout)

# --- Post-process: sort by semver, enrich with descriptions, diffs, commits ---

SEMVER_RE = re.compile(r'^v?(\d+)\.(\d+)\.(\d+)(.*)$')

def semver_key(version: str):
	"""Return sort key: numeric semver ascending."""
	m = SEMVER_RE.match(version)
	if m:
		return (0, int(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(4))
	return (1, version)

def semver_key_desc(version: str):
	"""Return sort key: numeric semver descending."""
	m = SEMVER_RE.match(version)
	if m:
		return (0, -int(m.group(1)), -int(m.group(2)), -int(m.group(3)), m.group(4))
	return (1, version)

def is_semver(version: str) -> bool:
	return SEMVER_RE.match(version) is not None

# Build tag2commit reverse mapping
tag2commit = {tag: commit for commit, tag in commit2tag.items()}

# Sort versions ascending by semver for diffing
all_versions = sorted(res.keys(), key=semver_key)

# Filter out vTEST for processing
semver_versions = [v for v in all_versions if v != 'vTEST' and is_semver(v)]

# Compute runner changes between consecutive versions
runner_changes_map = {}  # version -> {runner: {old, new}}
prev_runners = {}
for ver in semver_versions:
	runners = res[ver]
	changes = {}
	for rid, hashes in runners.items():
		old_hashes = prev_runners.get(rid)
		if old_hashes is None:
			# Runner newly introduced
			changes[rid] = {'old': None, 'new': hashes[0] if hashes else None}
		elif set(hashes) != set(old_hashes):
			changes[rid] = {'old': old_hashes[0] if old_hashes else None, 'new': hashes[0] if hashes else None}
	runner_changes_map[ver] = changes
	prev_runners = runners

# Collect git log between consecutive versions
commits_map = {}  # version -> [commit messages]
for i, ver in enumerate(semver_versions):
	if i == 0:
		commits_map[ver] = []
		continue
	prev_ver = semver_versions[i - 1]
	prev_commit_hash = tag2commit.get(prev_ver)
	curr_commit_hash = tag2commit.get(ver)
	if prev_commit_hash and curr_commit_hash:
		try:
			log_proc = subprocess.run(
				['git', 'log', '--oneline', f'{prev_commit_hash}..{curr_commit_hash}', '--', 'runners/'],
				capture_output=True,
				text=True,
				check=True,
			)
			commits_map[ver] = [
				line.split(None, 1)[1] if len(line.split(None, 1)) > 1 else line
				for line in log_proc.stdout.strip().splitlines()
				if line.strip()
			]
		except subprocess.CalledProcessError:
			commits_map[ver] = []
	else:
		commits_map[ver] = []

# Build enriched JSON
enriched_versions = []
for ver in sorted(semver_versions, key=semver_key_desc):
	entry = {
		'version': ver,
		'description': tag_messages.get(ver, ''),
		'runners': res[ver],
		'runner_changes': runner_changes_map.get(ver, {}),
		'commits': commits_map.get(ver, []),
	}
	enriched_versions.append(entry)

# Build per-runner version history
all_runner_ids = set()
for ver in semver_versions:
	all_runner_ids.update(res[ver].keys())

runners_history = {}
for rid in sorted(all_runner_ids):
	seen_hashes = []
	for ver in sorted(semver_versions, key=semver_key_desc):
		hashes = res[ver].get(rid)
		if hashes:
			h = hashes[0]
			if not seen_hashes or seen_hashes[-1]['hash'] != h:
				seen_hashes.append({'hash': h, 'introduced_in': ver})
	runners_history[rid] = {'versions': seen_hashes}

enriched_json = {
	'versions': enriched_versions,
	'runners': runners_history,
}

json_path = Path(sys.argv[1])
json_path.write_text(json.dumps(enriched_json, indent=2))

# --- RST generation helpers ---

def rst_heading(text, char):
	return f'{text}\n{char * len(text)}'

def make_table(rows, headers):
	"""Build an RST grid table from headers + rows (list of lists)."""
	col_widths = [len(h) for h in headers]
	for row in rows:
		for i, cell in enumerate(row):
			col_widths[i] = max(col_widths[i], max((len(l) for l in cell.split('\n')), default=0))

	def sep(ch='-'):
		return '+' + '+'.join(ch * (w + 2) for w in col_widths) + '+'

	def fmt_row(row):
		split_cells = [cell.split('\n') for cell in row]
		max_lines = max(len(c) for c in split_cells)
		lines = []
		for line_idx in range(max_lines):
			parts = []
			for i, cell_lines in enumerate(split_cells):
				text = cell_lines[line_idx] if line_idx < len(cell_lines) else ''
				parts.append(f' {text:<{col_widths[i]}} ')
			lines.append('|' + '|'.join(parts) + '|')
		return '\n'.join(lines)

	parts = [sep('-')]
	parts.append(fmt_row(headers))
	parts.append(sep('='))
	for row in rows:
		parts.append(fmt_row(row))
		parts.append(sep('-'))
	return '\n'.join(parts)

def version_to_anchor(ver):
	"""Convert version string to changelog anchor: v0.1.8 -> v0-1-8"""
	return ver.replace('.', '-')

# --- Runner tier definitions ---

TIER_1 = ['py-genlayer', 'py-genlayer-multi']
TIER_2 = ['py-lib-genlayer-embeddings']
TIER_3 = [
	'cpython', 'softfloat', 'py-lib-cloudpickle', 'py-lib-genlayer-std',
	'py-lib-protobuf', 'py-lib-word_piece_tokenizer', 'models-all-MiniLM-L6-v2',
]

RUNNER_DESCRIPTIONS = {
	'py-genlayer': 'Standard runner for single-file Python intelligent contracts. Includes CPython, the genlayer standard library, and all core dependencies.',
	'py-genlayer-multi': 'Runner for multi-file Python contract packages. Same dependencies as py-genlayer.',
	'py-lib-genlayer-embeddings': 'Adds semantic search and vector embedding support. Must be specified before py-genlayer in a Seq block.',
	'cpython': 'CPython 3.13 interpreter compiled to WebAssembly.',
	'softfloat': 'Software floating-point implementation for deterministic arithmetic.',
	'py-lib-cloudpickle': 'Object serialization library.',
	'py-lib-genlayer-std': 'GenLayer standard library providing the gl.* API.',
	'py-lib-protobuf': 'Protocol Buffers serialization.',
	'py-lib-word_piece_tokenizer': 'Tokenizer for embedding models.',
	'models-all-MiniLM-L6-v2': 'Pre-trained sentence embedding model (ONNX).',
}

# --- Generate runners-page_generated.rst ---

latest_ver = semver_versions[-1] if semver_versions else None
latest_runners = res.get(latest_ver, {}) if latest_ver else {}

def build_runner_version_list(rid):
	"""Build list of (hash, version_range_str) for previous versions of a runner.

	Groups consecutive versions sharing the same hash into ranges.
	Skips the current (latest) hash.
	"""
	history = runners_history.get(rid, {}).get('versions', [])
	if not history:
		return []
	# Skip the first entry (current hash)
	prev = history[1:]
	return prev

def format_version_range(rid):
	"""For each hash in the runner history (excluding current), compute the version range string."""
	# Walk through semver_versions ascending to group consecutive versions with same hash
	version_hash_pairs = []
	for ver in semver_versions:
		hashes = res[ver].get(rid)
		if hashes:
			version_hash_pairs.append((ver, hashes[0]))

	if not version_hash_pairs:
		return []

	# Group consecutive versions with same hash
	groups = []
	current_hash = version_hash_pairs[0][1]
	start_ver = version_hash_pairs[0][0]
	end_ver = version_hash_pairs[0][0]
	for ver, h in version_hash_pairs[1:]:
		if h == current_hash:
			end_ver = ver
		else:
			groups.append((current_hash, start_ver, end_ver))
			current_hash = h
			start_ver = ver
			end_ver = ver
	groups.append((current_hash, start_ver, end_ver))

	# The last group is the "current" hash, skip it
	prev_groups = groups[:-1]
	# Reverse so newest first
	prev_groups.reverse()
	return prev_groups

def render_runner_section(rid, rst_parts):
	desc = RUNNER_DESCRIPTIONS.get(rid, '')
	hashes = latest_runners.get(rid, [])
	current_hash = hashes[0] if hashes else 'N/A'

	rst_parts.append(rst_heading(rid, '^'))
	rst_parts.append('')
	if desc:
		rst_parts.append(desc)
		rst_parts.append('')
	rst_parts.append(f'Current hash: ``{current_hash}``')
	rst_parts.append('')

	prev_groups = format_version_range(rid)
	if prev_groups:
		rst_parts.append('.. raw:: html')
		rst_parts.append('')
		rst_parts.append('   <details>')
		rst_parts.append('   <summary>Previous versions</summary>')
		rst_parts.append('')
		for h, start_v, end_v in prev_groups:
			if start_v == end_v:
				anchor = version_to_anchor(start_v)
				rst_parts.append(f'- ``{h}`` \u2014 `{start_v} <changelog.html#{anchor}>`_')
			else:
				anchor = version_to_anchor(end_v)
				rst_parts.append(f'- ``{h}`` \u2014 {start_v} through `{end_v} <changelog.html#{anchor}>`_')
		rst_parts.append('')
		rst_parts.append('.. raw:: html')
		rst_parts.append('')
		rst_parts.append('   </details>')
		rst_parts.append('')

runners_rst = []

# Tier 1
runners_rst.append(rst_heading('Contract runners', '-'))
runners_rst.append('')
for rid in TIER_1:
	if rid in latest_runners:
		render_runner_section(rid, runners_rst)

# Tier 2
runners_rst.append(rst_heading('Optional runners', '-'))
runners_rst.append('')
for rid in TIER_2:
	if rid in latest_runners:
		render_runner_section(rid, runners_rst)

# Tier 3
runners_rst.append(rst_heading('Internal dependencies', '-'))
runners_rst.append('')
for rid in TIER_3:
	if rid in latest_runners:
		render_runner_section(rid, runners_rst)

# Also include any runners not in the tier lists
known_runners = set(TIER_1 + TIER_2 + TIER_3)
extra_runners = sorted(set(latest_runners.keys()) - known_runners)
if extra_runners:
	runners_rst.append(rst_heading('Other', '-'))
	runners_rst.append('')
	for rid in extra_runners:
		render_runner_section(rid, runners_rst)

runners_rst_path = json_path.with_name('runners-page_generated.rst')
runners_rst_path.write_text('\n'.join(runners_rst) + '\n')

# --- Generate changelog_generated.rst ---

# Notes dir is python-sdk/changelog-notes/.
# Sphinx resolves include paths relative to the top-level source file,
# which is python-sdk/changelog.rst (the file that includes our generated RST).
changelog_notes_dir = json_path.parent.parent.parent / 'python-sdk' / 'changelog-notes'
changelog_notes_include_prefix = 'changelog-notes'

changelog_rst = []

for entry in enriched_versions:
	ver = entry['version']
	desc = entry['description']
	runners = entry['runners']
	changes = entry['runner_changes']
	commits = entry['commits']

	# Find previous version for "no changes" message
	ver_idx = semver_versions.index(ver)
	prev_ver = semver_versions[ver_idx - 1] if ver_idx > 0 else None

	changelog_rst.append(f'.. _{version_to_anchor(ver)}:')
	changelog_rst.append('')
	changelog_rst.append(rst_heading(ver, '-'))
	changelog_rst.append('')
	if desc:
		changelog_rst.append(f'*{desc}*')
		changelog_rst.append('')

	# Check for hand-written notes — try exact version and base version (e.g. v0.1.8 for v0.1.8-runner-hash)
	notes_file = changelog_notes_dir / f'{ver}.rst'
	if not notes_file.exists():
		base_ver = SEMVER_RE.match(ver)
		if base_ver:
			base_name = f'v{base_ver.group(1)}.{base_ver.group(2)}.{base_ver.group(3)}'
			notes_file = changelog_notes_dir / f'{base_name}.rst'
	if notes_file.exists():
		rel_notes = f'{changelog_notes_include_prefix}/{notes_file.name}'
		changelog_rst.append(f'.. include:: {rel_notes}')
		changelog_rst.append('')

	if changes:
		changelog_rst.append('.. raw:: html')
		changelog_rst.append('')
		changelog_rst.append(f'   <details>')
		n = len(changes)
		s = 's' if n != 1 else ''
		changelog_rst.append(
			f'   <summary><strong>Runner Changes</strong> ({n} runner{s} updated)</summary>'
		)
		changelog_rst.append('')
		change_rows = []
		for rid in sorted(changes.keys()):
			ch = changes[rid]
			old_h = ch['old']
			new_h = ch['new']
			if old_h is None:
				change_rows.append([rid, '*new*', f'``{new_h}``'])
			else:
				change_rows.append([rid, f'``{old_h}``', f'``{new_h}``'])
		changelog_rst.append(make_table(change_rows, ['Runner', 'From', 'To']))
		changelog_rst.append('')

		if commits:
			changelog_rst.append('Commits touching runners:')
			changelog_rst.append('')
			for msg in commits:
				changelog_rst.append(f'- ``{msg}``')
			changelog_rst.append('')

		# Full runner hashes table
		changelog_rst.append('.. raw:: html')
		changelog_rst.append('')
		changelog_rst.append('   <details>')
		changelog_rst.append('   <summary>Full runner hashes</summary>')
		changelog_rst.append('')
		rows = []
		for rid in sorted(runners.keys()):
			hashes = runners[rid]
			hash_str = '\n'.join(f'``{h}``' for h in hashes)
			rows.append([rid, hash_str])
		if rows:
			changelog_rst.append(make_table(rows, ['Runner', 'Hash']))
			changelog_rst.append('')
		changelog_rst.append('.. raw:: html')
		changelog_rst.append('')
		changelog_rst.append('   </details>')
		changelog_rst.append('')

		changelog_rst.append('.. raw:: html')
		changelog_rst.append('')
		changelog_rst.append('   </details>')
		changelog_rst.append('')
	else:
		if prev_ver:
			changelog_rst.append(f'No runner changes from {prev_ver}.')
		else:
			changelog_rst.append('Initial release.')
		changelog_rst.append('')

changelog_rst_path = json_path.with_name('changelog_generated.rst')
changelog_rst_path.write_text('\n'.join(changelog_rst) + '\n')
