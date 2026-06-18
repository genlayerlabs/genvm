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

set -euo pipefail

git fetch origin --prune

remote_has() {
	git show-ref --verify --quiet "refs/remotes/origin/$1"
}

for V in $(python3 support/ci/branch-versions.py list); do
	VER="v${V}.x"
	DEV="v${V}-dev"

	if ! remote_has "$VER"; then
		echo "Creating version branch ${VER} from main"
		git push origin "origin/main:refs/heads/${VER}"
		git fetch origin "$VER"
	else
		echo "Version branch ${VER} already exists"
	fi

	if ! remote_has "$DEV"; then
		echo "Creating dev branch ${DEV} from ${VER}"
		git push origin "origin/${VER}:refs/heads/${DEV}"
		git fetch origin "$DEV"
	else
		echo "Dev branch ${DEV} already exists"
	fi

	existing="$(gh pr list --repo "$GITHUB_REPOSITORY" --base "$VER" --head "$DEV" --state open --json number --jq '.[0].number // ""')"
	if [ -z "$existing" ]; then
		echo "Opening standing release-gate PR ${DEV} -> ${VER}"
		gh pr create --repo "$GITHUB_REPOSITORY" --base "$VER" --head "$DEV" \
			--title "Release gate: ${DEV} → ${VER}" \
			--body "Standing release-gate PR. \`${DEV}\` accumulates incremental v${V} work; this PR merges into the release-ready \`${VER}\` branch only once the full cross-repo E2E matrix is green. It may stay red while the v${V} train is in progress."
	else
		echo "Standing PR ${DEV} -> ${VER} already open (#${existing})"
	fi
done
