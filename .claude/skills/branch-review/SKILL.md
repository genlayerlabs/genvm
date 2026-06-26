---
name: branch-review
description: Reviews the current GenVM branch by fanning out three specialized review agents (spec, security, implementation). Use when asked to "review this branch", "review the PR", or do a full code review of a diff.
---

# Reviewing a GenVM branch

Run a three-pass review using the dedicated subagents. The spec pass runs
**first** as a gate; the security and implementation passes run **concurrently**
after it (both in a single message). All three are read-only.

| Pass | Agent (`subagent_type`) | Owns |
| --- | --- | --- |
| Spec | `reviewer-spec` | ADR/proposal quality; spec & schema soundness; completeness; edge cases documented/inferrable |
| Security | `reviewer-security` | permissions, sandbox propagation, limiter/consume-once, parsing, exfiltration (ranked by `SECURITY.md`) |
| Implementation | `reviewer-implementation` | code-vs-spec drift, AI slop, duplicated logic, edge cases tested, doc completeness, useless comments |

## Baseline

All three diff against the active **dev** branch, not `main` (default
`v0.3-dev`; confirm with `git branch -a | grep dev`). Pass the base in the prompt
if the user named one. Do NOT treat the dev-mode / `hashes=test` build state as a
finding — it is intentional.

## Procedure

1. Confirm the base branch (default `v0.3-dev`).
2. Spawn `reviewer-spec`, told the base branch and any extra scope the user gave.
   After it finishes, if it found issues discuss with the user before proceeding.
3. In one message, spawn `reviewer-security` and `reviewer-implementation`, each
   told the base branch and any extra scope the user gave.
4. Synthesize their three reports into one review: lead with the merge verdict,
   then a **Blocking** section and a **Nits** section, attributing findings to
   their pass. De-duplicate overlaps (e.g. a duplicated-logic issue both security
   and implementation raise) into a single entry.

## Scope notes

- If the user asks for only one dimension ("just the security review"), run that
  one agent instead of all three.
- Each agent is read-only; none will edit code. If the user wants fixes after the
  review, that is a separate, explicit follow-up.
