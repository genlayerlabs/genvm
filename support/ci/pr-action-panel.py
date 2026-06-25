#!/usr/bin/env python3
"""Handle a ticked box on the GenVM PR action panel.

Invoked by .github/workflows/branch_pr_actions.yaml on issue_comment:edited
when the edited comment is the panel (it carries the `<!-- genvm-actions -->`
marker). Steps:

1. ignore edits made by a bot (the panel's own reset echo);
2. do nothing unless the PR carries the `ci-safe` label (the gate that lets
any of these actions run; auto-added for write-access authors);
3. parse which boxes are ticked and act:
- "Force run full tests" is a STICKY toggle: when ticked we just ensure the
`run-full-tests` label is set (so queue.yaml runs on every push). We neither
untick it nor trigger a one-off run — its checked state mirrors the label.
- "Rerun full tests" is a momentary button: re-apply `run-full-tests`
(remove+add) to force a fresh queue.yaml run on the current head, then untick.
- "Merge" is a momentary button: expose `merge=true` so the caller runs the
reusable merge workflow, then untick.
4. untick the momentary boxes (not the sticky Force one).

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


def remove_label(name):
	gh(
		'api', '--method', 'DELETE', f'repos/{REPO}/issues/{PR}/labels/{name}', check=False
	)


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

	# Force: sticky enable of the run-full-tests marker (no untick, no one-off).
	if any('force' in b for b in boxes) and RUN_FULL_TESTS_LABEL not in current:
		add_label(RUN_FULL_TESTS_LABEL)

	# Rerun: force a fresh queue run on the current head even if the marker is
	# already set (remove+add re-emits the `labeled` event).
	if any('rerun' in b for b in boxes):
		remove_label(RUN_FULL_TESTS_LABEL)
		add_label(RUN_FULL_TESTS_LABEL)

	if any('merge' in b for b in boxes):
		set_output('merge', 'true')

	untick_momentary(body)


if __name__ == '__main__':
	main()
