#!/usr/bin/env python3
"""Gate and perform a `/merge` of a PR into a v<X>-dev branch.

Invoked by .github/workflows/branch_merge_into_dev.yaml on a maintainer
`/merge` comment. It re-checks every gate against the EXACT PR head commit
and then advances the dev branch by a plain (fast-forward-only) push, so
what lands is byte-identical to what CI and E2E validated.

Gates (all required, all on the head commit):
1. base branch is a v<X>-dev branch
2. PR carries the `rtm` (ready-to-merge) label
3. full GenVM CI (queue.yaml) concluded success
4. the cross-repo E2E check concluded success
5. the PR is 0 commits behind base

Strategy: 1 commit -> fast-forward the original commit (SHA preserved);
more -> squash into one commit on top of base, then fast-forward. The PR
is closed afterwards.

Talks to GitHub through the `gh` CLI and moves refs through `git`; it
never executes PR code. The dev branches are protected, so the workflow
checks out with the GENVM_CI_PRIVATE_KEY deploy key and pushes non-force
(a base that advanced is safely rejected).

Env: GITHUB_REPOSITORY, PR_NUMBER, GH_TOKEN, E2E_CHECK_PATTERN, COMMENT_ID.
"""

import json
import os
import re
import subprocess
import sys

REPO = os.environ['GITHUB_REPOSITORY']
PR = os.environ['PR_NUMBER']
E2E_PATTERN = os.environ.get('E2E_CHECK_PATTERN', 'e2e')
COMMENT_ID = os.environ.get('COMMENT_ID')


def run(*args, check=True):
	return subprocess.run(args, check=check, text=True, capture_output=True)


def gh(*args):
	return run('gh', *args).stdout


def git(*args, check=True):
	return run('git', *args, check=check)


def block(msg):
	"""Comment the reason on the PR and fail the job."""
	run(
		'gh',
		'pr',
		'comment',
		PR,
		'--repo',
		REPO,
		'--body',
		f'❌ `/merge` blocked: {msg}',
		check=False,
	)
	sys.exit(1)


def pr_view(*fields):
	out = gh('pr', 'view', PR, '--repo', REPO, '--json', ','.join(fields))
	return json.loads(out)


def check_gates(pr):
	if pr['state'] != 'OPEN':
		block(f"PR is not open (state: {pr['state']}).")
	if pr['isDraft']:
		block('PR is a draft.')

	base = pr['baseRefName']
	if not re.fullmatch(r'v.*-dev', base):
		block(f'base branch `{base}` is not a `v<X>-dev` branch.')

	# 2. rtm label
	if not any(label['name'] == 'rtm' for label in pr['labels']):
		block('missing the `rtm` (ready-to-merge) label.')

	head_sha = pr['headRefOid']

	# 3. full GenVM CI (queue.yaml) green on the head commit
	runs = json.loads(
		gh(
			'api',
			f'repos/{REPO}/actions/workflows/queue.yaml/runs'
			f'?head_sha={head_sha}&event=pull_request',
		)
	)['workflow_runs']
	latest = runs[0] if runs else None
	if not latest or latest['status'] != 'completed' or latest['conclusion'] != 'success':
		got = 'no run' if not latest else f"{latest['status']} {latest['conclusion']}"
		block(f'GenVM CI (queue.yaml) is not green on `{head_sha}` (got: {got}).')

	# 4. cross-repo E2E check green on the head commit
	checks = json.loads(
		gh(
			'api',
			f'repos/{REPO}/commits/{head_sha}/check-runs?per_page=100',
		)
	)['check_runs']
	e2e = [c for c in checks if re.search(E2E_PATTERN, c['name'], re.I)]
	if not e2e:
		block(f'no E2E check found on `{head_sha}` (run it on this PR first).')
	bad = [c for c in e2e if c['conclusion'] != 'success']
	if bad:
		conclusions = ' '.join(c['conclusion'] or 'pending' for c in e2e)
		block(f'E2E is not green on `{head_sha}` (conclusions: {conclusions}).')

	return base, head_sha


def merge(pr, base, head_sha):
	# Fetch the exact head commit (works for fork PRs too) and live base tip.
	git(
		'fetch',
		'--no-tags',
		'origin',
		f'refs/pull/{PR}/head:refs/prhead',
		f'+refs/heads/{base}:refs/remotes/origin/{base}',
	)

	fetched = git('rev-parse', 'refs/prhead').stdout.strip()
	if fetched != head_sha:
		block(
			f'head moved during merge (expected `{head_sha}`, got `{fetched}`); re-run /merge.'
		)

	# 5. authoritative 0-commits-behind check at merge time.
	if (
		git(
			'merge-base', '--is-ancestor', f'origin/{base}', 'refs/prhead', check=False
		).returncode
		!= 0
	):
		behind = git('rev-list', '--count', f'refs/prhead..origin/{base}').stdout.strip()
		block(
			f'PR is {behind} commit(s) behind `{base}`; update the branch and re-run /merge.'
		)

	git('config', 'user.name', 'genvm-ci')
	git('config', 'user.email', 'genvm-ci@genlayer.com')

	if len(pr['commits']) == 1:
		print(f'Single commit: fast-forwarding {base} to {head_sha}')
		push_sha = head_sha
	else:
		print(f"Squashing {len(pr['commits'])} commits onto {base}")
		author = git('log', '-1', '--format=%an <%ae>', head_sha).stdout.strip()
		git('checkout', '-B', '_merge', f'origin/{base}')
		git('merge', '--squash', head_sha)
		message = f"{pr['title']} (#{PR})\n\n{pr['body'] or ''}\n"
		git('commit', '--author', author, '-m', message)
		push_sha = git('rev-parse', 'HEAD').stdout.strip()

	# Non-force FF push; rejected if base advanced since the checks.
	if (
		git('push', 'origin', f'{push_sha}:refs/heads/{base}', check=False).returncode != 0
	):
		block(f'fast-forward push to `{base}` was rejected (base advanced); re-run /merge.')

	run(
		'gh',
		'pr',
		'comment',
		PR,
		'--repo',
		REPO,
		'--body',
		f'✅ Merged into `{base}` (`{push_sha}`) via fast-forward.',
		check=False,
	)
	run('gh', 'pr', 'close', PR, '--repo', REPO, check=False)


def main():
	if COMMENT_ID:
		run(
			'gh',
			'api',
			'--method',
			'POST',
			f'repos/{REPO}/issues/comments/{COMMENT_ID}/reactions',
			'-f',
			'content=eyes',
			check=False,
		)

	pr = pr_view(
		'baseRefName',
		'headRefName',
		'headRefOid',
		'commits',
		'labels',
		'state',
		'isDraft',
		'title',
		'body',
	)
	base, head_sha = check_gates(pr)
	merge(pr, base, head_sha)


if __name__ == '__main__':
	main()
