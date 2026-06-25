#!/usr/bin/env python3
"""Handle a ticked box on the GenVM PR action panel.

Invoked by .github/workflows/branch_pr_actions.yaml on issue_comment:edited
when the edited comment is the panel (it carries the `<!-- genvm-actions -->`
marker). Steps:

1. ignore edits made by a bot (the panel's own reset echo);
2. do nothing unless the PR carries the `ci-safe` label (the gate that lets
any of these actions run; auto-added for write-access authors);
3. parse which boxes are ticked and act:
- "Force run full tests" is a STICKY toggle: on the edit that newly ticks it
we set the `run-full-tests` label (so queue.yaml runs on every future push)
AND dispatch one run now. We do not untick it — its checked state mirrors the
label, and an already-set label means no re-dispatch on unrelated edits.
- "Rerun full tests" is a momentary button: dispatch a fresh queue.yaml run on
the current head, then untick.
- "Merge" is a momentary button: expose `merge=true` so the caller runs the
reusable merge workflow, then untick.
4. untick the momentary boxes (not the sticky Force one).

Runs are started with `gh workflow run queue.yaml --ref <PR head branch>` (a
`workflow_dispatch`), NOT by toggling a label: a label applied by the bot's
GITHUB_TOKEN does not emit a `labeled` event (GitHub blocks recursive runs),
whereas a `workflow_dispatch` from the same token does run. Dispatching on the
PR head branch makes the run's head_sha equal the PR head, so the Merge gate
counts it. (Fork PRs have no head branch in this repo, so the panel can only
dispatch for same-repo branches — the common GenVM flow.)

Resetting the panel re-fires issue_comment:edited, but that event is sent by
the bot, so it is ignored — no loop.

Env: GITHUB_REPOSITORY, PR_NUMBER, COMMENT_ID, SENDER, GH_TOKEN.
"""

import os
import re
import subprocess

REPO = os.environ['GITHUB_REPOSITORY']
PR = os.environ['PR_NUMBER']
COMMENT_ID = os.environ['COMMENT_ID']
SENDER = os.environ['SENDER']

CI_SAFE_LABEL = 'ci-safe'
RUN_FULL_TESTS_LABEL = 'run-full-tests'


def run(*args, check=True):
	return subprocess.run(args, check=check, text=True, capture_output=True)


def gh(*args, check=True):
	return run('gh', *args, check=check)


def set_output(name, value):
	with open(os.environ['GITHUB_OUTPUT'], 'a') as f:
		f.write(f'{name}={value}\n')


def labels():
	out = gh('api', f'repos/{REPO}/issues/{PR}/labels', '--jq', '.[].name').stdout
	return set(out.splitlines())


def add_label(name):
	gh(
		'api',
		'--method',
		'POST',
		f'repos/{REPO}/issues/{PR}/labels',
		'-f',
		f'labels[]={name}',
	)


def head_branch():
	return gh(
		'pr', 'view', PR, '--repo', REPO, '--json', 'headRefName', '--jq', '.headRefName'
	).stdout.strip()


def dispatch_full_tests():
	# Start queue.yaml on the PR head branch so the run's head_sha matches the
	# PR head (the Merge gate keys off head_sha). A workflow_dispatch fired with
	# GITHUB_TOKEN does create a run, unlike a bot-applied `labeled` event.
	branch = head_branch()
	if not branch:
		print('could not resolve PR head branch; cannot dispatch full tests')
		return
	gh('workflow', 'run', 'queue.yaml', '--repo', REPO, '--ref', branch, '-f', f'pr={PR}')
	print(f'dispatched queue.yaml on `{branch}`')


def ticked_boxes(body):
	return [
		m.group(1).strip().lower()
		for m in re.finditer(r'(?m)^\s*-\s*\[[xX]\]\s*(.+?)\s*$', body)
	]


def untick_momentary(body):
	# Untick every ticked box EXCEPT the sticky "Force run full tests" one.
	def reset(line):
		if re.match(r'\s*-\s*\[[xX]\]', line) and 'force' not in line.lower():
			return re.sub(r'\[[xX]\]', '[ ]', line, count=1)
		return line

	new = '\n'.join(reset(line) for line in body.splitlines())
	if body.endswith('\n'):
		new += '\n'
	if new != body:
		gh(
			'api',
			'--method',
			'PATCH',
			f'repos/{REPO}/issues/comments/{COMMENT_ID}',
			'-f',
			f'body={new}',
		)


def main():
	set_output('merge', 'false')

	# Ignore the bot's own panel-reset edit (avoids a self-trigger loop).
	if SENDER.endswith('[bot]'):
		print(f'{SENDER} is a bot (panel reset echo); ignoring')
		return

	current = labels()
	if CI_SAFE_LABEL not in current:
		print(f'PR lacks the `{CI_SAFE_LABEL}` label; ignoring panel actions')
		return

	body = gh('api', f'repos/{REPO}/issues/comments/{COMMENT_ID}', '--jq', '.body').stdout
	boxes = ticked_boxes(body)
	if not boxes:
		print('no ticked boxes; nothing to do')
		return

	# Force: sticky enable of the run-full-tests marker so every future push
	# runs full tests. Only on the edit that newly sets it do we also dispatch
	# a run now (an already-set label means this is an unrelated edit -> no
	# duplicate run).
	if any('force' in b for b in boxes) and RUN_FULL_TESTS_LABEL not in current:
		add_label(RUN_FULL_TESTS_LABEL)
		dispatch_full_tests()

	# Rerun: momentary -> always dispatch a fresh run on the current head.
	if any('rerun' in b for b in boxes):
		dispatch_full_tests()

	if any('merge' in b for b in boxes):
		set_output('merge', 'true')

	untick_momentary(body)


if __name__ == '__main__':
	main()
