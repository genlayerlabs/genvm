# TODO

## High Priority

- [ ] Explore executor host functions (`executor/src/host/`) for non-determinism vectors
- [ ] Explore WASI implementation for filesystem/clock/random syscalls that could cause divergence
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
