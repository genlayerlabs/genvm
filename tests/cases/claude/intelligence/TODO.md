# TODO

## Harness blockers (discovered 2026-06-12)

- `.direnv/ya-test-runner` is STALE: its `Description` lacks the `depends_on`/`hidden`
  fields the repo source (`tools/ya-test-runner`) already has, so collection throws
  `TypeError: Description.__new__() got an unexpected keyword argument 'depends_on'`.
  This breaks ALL integration-test collection. `run_test.py` itself is correct.
  Proper fix: rebuild the runner via nix/direnv. Workaround used this session:
  run the inner wrapped python with `PYTHONPATH=tools/ya-test-runner` shadowing the
  installed package — collection then works, but `--filter-name`/`--filter-tag` do
  NOT reliably isolate a single test (mixed repo/installed modules), so the suite
  runs whole.
- Running the whole suite drags in `cargo-afl` rust fuzz targets that abort at link
  (`cc … signal 6 SIGABRT`) and web tests with no webdriver, so genvm integration
  leader steps end up with only `config.json` and no execution.
- Net: could not get a clean deterministic `lvs` run for `agent/float_math` this
  session. Re-run after rebuilding `.direnv/ya-test-runner` (manager up on :3999,
  modules in user_error mode via `run-manager.sh`).
- Manager dies across tool calls; start it and run the test in the SAME shell call.
  Do NOT `pkill -f genvm-modules` — it matches the shell's own command line and
  suicides; use `pkill -x genvm-modules`.

## High Priority

- [ ] Explore executor host functions (`executor/src/host/`) for non-determinism vectors
- [x] Explore WASI implementation for filesystem/clock/random syscalls that could cause divergence (all deterministic - MT19937 rng, fixed clocks, FNV hash, deterministic memory)
- [ ] Test floating point operations across leader/validators for determinism
- [ ] Test memory allocation patterns that might differ across runs

## Medium Priority

- [ ] Explore Python runner (`runners/genlayer-py-std/`) for non-deterministic built-ins
- [ ] Test exception handling edge cases in contract calls
- [ ] Test storage read/write ordering under concurrent-like scenarios
- [ ] Test cross-contract call edge cases (recursion, reentrancy)

## Low Priority

- [ ] Test large contract deployments and memory limits
- [ ] Test calldata parsing edge cases (malformed JSON, unicode, etc.)
- [ ] Test gas/resource exhaustion boundaries
