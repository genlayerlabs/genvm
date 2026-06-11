# Security Policy

GenVM is consensus-critical: it executes Intelligent Contracts and its output must be
identical and trustworthy across all validators. Please treat security issues accordingly.

## Reporting a vulnerability

**Do not open a public issue.** Report privately via GitHub's
[private vulnerability reporting](https://github.com/genlayerlabs/genvm/security/advisories/new),
or email kira@yeager.ai

Include a description, affected component/version, and a reproduction (a contract, calldata,
or test case) where possible. We aim to acknowledge within a few business days.

## Severity priorities

Issues are triaged by impact, highest first:

1. **Remote code execution** — escaping the WASM/VM sandbox, or running attacker code in a
   validator, the manager/modules, the build pipeline, or release artifacts.
2. **Determinism violation / financial issues** — anything that makes honest validators
   diverge or accept invalid results (non-canonical encoding, version-gate bypass, untrusted
   caches, balance/fee accounting), or that lets a contract overspend or misaccount funds.
3. **Undefined behavior** — out-of-bounds reads/writes, type confusion, and other UB at the
   native (Rust `unsafe`, C extension) boundary, even without a known exploit yet.
4. **Crash / internal error** — contract-triggerable panics, aborts, or unhandled errors that
   should instead be a canonical `VMError` (availability impact).
5. **Miscellaneous** — secret/credential leakage in logs, info disclosure, resource exhaustion
   with bounded impact, and hardening gaps.

A native out-of-bounds access that reads/corrupts memory is UB (priority 3); a checked Rust
bounds-check that merely panics is a crash (priority 4).

## Scope

In scope: the executor, runners/SDK, modules (LLM/web/manager), the install/manifest path, and
the CI/release supply chain. Out of scope: issues requiring a pre-compromised host or operator
machine, and non-default deployments that expose the loopback-only manager to untrusted networks
(though we still want to know).
