# TODO

## Harness notes (discovered 2026-06-12)

- RUN VIA `nix develop .#full`, not `run_test.py`. The `.direnv/ya-test-runner` is
  STALE (its `Description` lacks `depends_on`/`hidden`), so `run_test.py` — which
  hardcodes that binary — fails collection with
  `TypeError: Description.__new__() got an unexpected keyword argument 'depends_on'`.
  The nix `full` shell puts a freshly-built `ya-test-runner` on PATH that works.
  Invoke `ya-test-runner --filter-name <id> run --no-manager --no-webdriver` directly.
- `--filter-name`/`--filter-tag` do NOT isolate a single test — the suite runs whole
  (718 pass / 78 fail; the 78 are LLM/web tests that need real modules, run here in
  user_error mode). Just read the specific test's `✓`/`✗` lines and its artifacts.
- Manager dies across tool calls; start it and run the test in the SAME shell call.
  Do NOT `pkill -f genvm-modules` — it matches the shell's own command line and
  suicides; use `pkill -x genvm-modules`.

## High Priority

- [ ] Explore executor host functions (`executor/src/host/`) for non-determinism vectors
- [x] Explore WASI implementation for filesystem/clock/random syscalls that could cause divergence (all deterministic - MT19937 rng, fixed clocks, FNV hash, deterministic memory)
- [ ] Test floating point operations across leader/validators for determinism
- [ ] Test memory allocation patterns that might differ across runs

## Medium Priority

- [ ] Test exception handling edge cases in contract calls
- [ ] Test storage read/write ordering under concurrent-like scenarios
- [ ] Test cross-contract call edge cases (recursion, reentrancy)

## Low Priority

- [ ] Test large contract deployments and memory limits
- [ ] Test calldata parsing edge cases (malformed JSON, unicode, etc.)
- [ ] Test gas/resource exhaustion boundaries
