# float_math probe

Determinism probe for floating-point / transcendental math (`sin`, `cos`, `exp`,
`sqrt`, `log`, `atan2`, `**`) plus `repr()` of floats — the classic place a hardware
FPU's last bit leaks per-machine nondeterminism. In det mode these must resolve via
the deterministic softfloat path, so leader/validator/sync hashes must agree.

Status: VERIFIED deterministic (2026-06-12) via `nix develop .#full`. Leader,
validator, and sync produce byte-identical hashes; `compute()` returns
`453.77516336575655|3.141592653589793|1.4142135623730951|0.30000000000000004`
on all replicas. No divergence — softfloat path holds. Not a discovery.

How to run (the stale `.direnv/ya-test-runner` can't; use the nix shell):

    nix develop .#full --command bash -c '
      build/out/bin/genvm-modules manager --port 3999 --host 127.0.0.1 --reroute-to vTEST &
      # start Llm + Web modules in user_error mode (see ../../run-manager.sh)
      ya-test-runner --filter-name claude/agent/float_math run --no-manager --no-webdriver'
