# float_math probe

Determinism probe for floating-point / transcendental math (`sin`, `cos`, `exp`,
`sqrt`, `log`, `atan2`, `**`) plus `repr()` of floats — the classic place a hardware
FPU's last bit leaks per-machine nondeterminism. In det mode these must resolve via
the deterministic softfloat path, so leader/validator/sync hashes must agree.

Status: written but NOT verified this session — the test runner is blocked (see
`../../intelligence/TODO.md`, "Harness blockers"). Re-run once `.direnv/ya-test-runner`
is rebuilt. No determinism violation observed (the run never produced a leader hash
to compare), so this is not a discovery.
