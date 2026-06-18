# GenVM branch model

GenVM uses an integration/release split per version train, mirroring the
rest of the v0.6 fee train. Active trains are declared in
[`.genvm-monorepo-root`](../../.genvm-monorepo-root) under
`active-versions` (e.g. `["0.3"]`).

For each active version `X`:

| Branch     | Role                                                         | Protected |
|------------|--------------------------------------------------------------|-----------|
| `v<X>-dev` | Integration. All work lands here; may be red during a train. | yes       |
| `v<X>.x`   | Release-ready. Only updated via the standing release-gate PR.| yes       |
| `main`     | Default branch; an **alias** of the latest `v<X>.x`.         | yes       |

## Flow

```
feature branch ──PR──▶ v<X>-dev ──standing PR (E2E gate)──▶ v<X>.x ──fast-forward──▶ main (latest train only)
```

- **Contributions** target `v<X>-dev`. A PR opened against `main` is
  retargeted to the latest dev branch automatically
  (`branch_retarget.yaml`).
- **Merge queue** may only merge into a `v<X>-dev` branch
  (`branch_queue_guard.yaml` fails otherwise). Normal GenVM CI
  (`queue.yaml`) runs in the queue as before.
- **Release gate**: a standing PR `v<X>-dev → v<X>.x` stays open. It is
  gated by the cross-repo E2E pipeline owned by
  [`genlayerlabs/genlayer-e2e`](https://github.com/genlayerlabs/genlayer-e2e)
  (synced into this repo as `.github/workflows/e2e.yml`; **not** a
  `branch_*` workflow — see below). A maintainer comments
  `/run-e2e <track>` on the standing PR to fire it; the pipeline posts a
  check-run that branch protection on `v<X>.x` requires, so the PR merges
  only once E2E is green.
- **main** is never committed to directly. On every push to the latest
  version branch, `branch_forward.yaml` fast-forwards `main` to it, so
  `main` always equals `v<latest>.x`.
- **Provisioning**: `branch_provision.yaml` creates any missing version
  branch, dev branch, and standing PR for every entry in
  `active-versions` (idempotent).

## Workflows (all `branch_*`)

| File                       | Trigger                              | Does                                          |
|----------------------------|--------------------------------------|-----------------------------------------------|
| `branch_forward.yaml`      | push to `v*.x`                       | fast-forward `main` to the latest `v<X>.x`    |
| `branch_retarget.yaml`     | PR opened/reopened against `main`    | retarget to `v<latest>-dev` + comment         |
| `branch_provision.yaml`    | dispatch / `active-versions` change  | create dev/version branches + standing PRs    |
| `branch_queue_guard.yaml`  | `merge_group`                        | fail unless the queue target is `v<X>-dev`    |
| `branch_new_version.yaml`  | dispatch                             | cut the next train; push `main` + `v<new>.x` + `v<new>-dev` |

The E2E release gate is **not** a `branch_*` workflow. The pipeline
(`.github/workflows/e2e.yml`) and its cache/artifact housekeeper
(`.github/workflows/e2e-housekeeper.yml`) are source-of-truth templates
owned by `genlayerlabs/genlayer-e2e` and synced into this repo
byte-identically via its sync PRs — GenVM is registered in that repo's
`repos.yaml` (`component: genvm`, `gate_policy: release-branches`). Don't
hand-edit them here; changes ship as a sync PR from genlayer-e2e. They run
on a `/run-e2e` PR comment, not on the branch model's events.

`support/ci/branch-versions.py` reads `active-versions` (`list`/`latest`;
honors a `MONOREPO_ROOT` env override). `support/ci/provision-branches.sh`
is the provisioning logic.

### Cutting a new version train

`branch_new_version.yaml` (Actions → run, pick `minor`/`major` or set an
explicit version) does it end to end: `check-versions.py bump` raises
`major-minor`, appends to `active-versions`, and sets the crate
[package] versions to `<new>.0`; the commit is pushed to `main`,
`v<new>.x` and `v<new>-dev`. The `v<new>-dev` push then trips
`branch_provision`, which opens the standing release-gate PR.

> `.genvm-monorepo-root`'s `major-minor` is unrelated to
> `active-versions`: it pins the single major.minor that *this branch's*
> crates must match. The `check-versions.py` pre-commit hook is a
> **fixer** — it rewrites the crate `[package]` versions to that
> major.minor (patch kept) and fails if it had to, so you re-stage. On
> `v0.3-dev` and `v0.3.x` it is `0.3`.

## One-time setup runbook (repo admin)

These are live, irreversible operations — run them yourself with an admin
token. `<X>` = latest active version (`0.3` today).

```sh
# 0. Deploy key — branch_forward / branch_provision push over SSH.
#    Add the GENVM_CI_PRIVATE_KEY public key as a repo deploy key with
#    write access, and put it on each protected branch ruleset's bypass
#    list (steps 3-5). (Already configured for the old fast-forward flow.)

# 1. Create the version + dev branches and the standing PR.
#    Either run the branch_provision workflow from the Actions tab, or:
git fetch origin
git push origin "origin/main:refs/heads/v${X}.x"      # if absent
git push origin "origin/v${X}.x:refs/heads/v${X}-dev" # if absent
gh pr create --base "v${X}.x" --head "v${X}-dev" \
  --title "Release gate: v${X}-dev → v${X}.x" \
  --body "Standing release-gate PR."

# 2. Make v<X>-dev the default branch (main stays, as an alias).
gh repo edit --default-branch "v${X}-dev"
```

### Branch protection (all three protected)

For `v<X>-dev`, `v<X>.x`, and `main` create a ruleset (or classic
protection) that:

- requires PRs (no direct pushes),
- bars force-push and deletion,
- bypass: the `GENVM_CI_PRIVATE_KEY` deploy key (so `branch_forward` /
  `branch_provision` can push).

Required status checks:

- `v<X>-dev`: normal GenVM CI via the **merge queue** (`queue.yaml`) plus
  `branch / merge-queue target guard`. Enable the merge queue for this
  branch.
- `v<X>.x`: the genlayer-e2e E2E check-run, posted when a maintainer runs
  `/run-e2e <track>` on the standing PR (select it as a required check
  once the first run surfaces it in the checks list). This PR may stay red
  while the train is in progress; it merges only when E2E is green.
- `main`: protected, fast-forward-only by the deploy key; no required
  checks needed (content already validated upstream).

### Retiring `main` as a development target

`main` is **not deleted** — it remains the default-visible alias of the
latest release branch. Just ensure nothing pushes to it directly; the
`branch_retarget` workflow moves stray PRs to the dev branch.

## Adding a new version train (e.g. 0.6)

1. Add `"0.6"` to `active-versions` in `.genvm-monorepo-root` (on
   `main` / the dev branches; keep it consistent across branches).
2. `branch_provision` creates `v0.6.x`, `v0.6-dev`, and the standing PR.
3. Once `0.6` is the highest active version, `branch_forward` starts
   aliasing `main` to `v0.6.x`; flip the default branch to `v0.6-dev`.
