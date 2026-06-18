#!/usr/bin/env bash

# Provision the branch model for every active version train.
#
# For each version X in .genvm-monorepo-root's `active-versions`:
#   1. ensure the version branch v<X>.x exists (created from main if absent)
#   2. ensure the dev branch v<X>-dev exists (created from v<X>.x if absent)
#   3. ensure a standing release-gate PR v<X>-dev -> v<X>.x is open
#
# Branch creation pushes over SSH using the GENVM_CI_PRIVATE_KEY deploy
# key (the checkout step wires it up), so it works even though the
# version/dev branches are protected. PR creation uses GH_TOKEN.
#
# Idempotent: re-running only creates what is missing.

set -x -euo pipefail

git fetch origin --prune

remote_has() {
	git show-ref --verify --quiet "refs/remotes/origin/$1"
}

versions="$(python3 support/ci/branch-versions.py list)"

# Collect every missing branch as a refspec and create them all in one
# atomic push, so we never leave a train half-provisioned.
pushrefs=()

for V in $versions; do
	VER="v${V}.x"
	DEV="v${V}-dev"

	if ! remote_has "$VER"; then
		echo "Will create version branch ${VER} from main"
		ver_commit="$(git rev-parse origin/main)"
		pushrefs+=("origin/main:refs/heads/${VER}")
	else
		echo "Version branch ${VER} already exists"
		ver_commit="$(git rev-parse "origin/${VER}")"
	fi

	if ! remote_has "$DEV"; then
		# Seed the dev branch one (empty) commit ahead of the version branch:
		# a fresh ${DEV} identical to ${VER} has no diff, so the standing
		# release-gate PR below would fail with "No commits between".
		echo "Will create dev branch ${DEV} from ${VER} (with seed commit)"
		seed="$(GIT_AUTHOR_NAME='github-actions[bot]' \
			GIT_AUTHOR_EMAIL='41898282+github-actions[bot]@users.noreply.github.com' \
			GIT_COMMITTER_NAME='github-actions[bot]' \
			GIT_COMMITTER_EMAIL='41898282+github-actions[bot]@users.noreply.github.com' \
			git commit-tree "${ver_commit}^{tree}" -p "${ver_commit}" \
			-m "chore(ci): seed ${DEV} release-gate branch 🌱")"
		pushrefs+=("${seed}:refs/heads/${DEV}")
	else
		echo "Dev branch ${DEV} already exists"
	fi
done

if [ "${#pushrefs[@]}" -gt 0 ]; then
	echo "Creating ${#pushrefs[@]} branch(es) in one atomic push"
	git push --atomic origin "${pushrefs[@]}"
	git fetch origin --prune
else
	echo "All branches already exist; nothing to push"
fi

for V in $versions; do
	VER="v${V}.x"
	DEV="v${V}-dev"

	existing="$(gh pr list --repo "$GITHUB_REPOSITORY" --base "$VER" --head "$DEV" --state open --json number --jq '.[0].number // ""')"
	if [ -z "$existing" ]; then
		echo "Opening standing release-gate PR ${DEV} -> ${VER}"
		gh pr create --repo "$GITHUB_REPOSITORY" --base "$VER" --head "$DEV" \
			--title "Release gate: ${DEV} → ${VER}" \
			--body "Standing release-gate PR. \`${DEV}\` accumulates incremental v${V} work; this PR merges into the release-ready \`${VER}\` branch only once the cross-repo E2E pipeline is green. Comment \`/run-e2e v${V}\` to fire it (genlayerlabs/genlayer-e2e). It may stay red while the v${V} train is in progress."
	else
		echo "Standing PR ${DEV} -> ${VER} already open (#${existing})"
	fi
done
