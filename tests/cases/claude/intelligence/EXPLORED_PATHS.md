# Explored Paths

## WASI Implementation (executor/src/wasi/)

### Files Explored
- `executor/src/wasi/preview1.rs` - All WASI Preview1 syscalls
- `executor/src/wasi/vfs.rs` - Virtual filesystem implementation
- `executor/src/wasi/genlayer_sdk.rs` - GenVM extension functions
- `executor/src/wasi/base.rs` - Config struct (is_deterministic, permissions)
- `executor/src/wasi/mod.rs` - WASI module initialization
- `executor/src/lib.rs` - Main executor, always sets is_deterministic=true
- `executor/src/rt/supervisor/mod.rs` - Wasmtime engine config (floats disabled, NaN canonicalization)

### Findings - All Deterministic
1. **`random_get`**: Uses MT19937 with hardcoded seed `[GenL, ayer]` in deterministic mode
2. **`clock_time_get`**: Returns fixed timestamp from blockchain message
3. **All clocks** (monotonic, perf_counter, etc.): Map to same fixed timestamp
4. **`process_time`**: Returns 0
5. **VFS**: Read-only, immutable content from pre-mapped Bytes
6. **`fd_readdir`**: BTreeMap ensures sorted order
7. **`environ_get`/`args_get`**: Pre-populated, fixed values
8. **Sockets**: All blocked (EACCES)
9. **Write ops**: All blocked (EROFS)
10. **`proc_exit`**: Deterministic

### Python Runtime Observations
- `PYTHONHASHSEED`: Not set, but Python uses FNV 32-bit hash (no randomization)
- `hash()` on strings/bytes: Deterministic across runs
- `set` iteration order: Deterministic (follows from hash determinism)
- `id()`/`repr()`: Memory addresses identical across runs (WASM deterministic memory)
- `os.getpid()`: Fixed at 42
- `sys.platform`: "wasi"
- Only 3 env vars: PYTHONHOME, PYTHONPATH, pwd

### Tests Created (all pass, no divergence found)
- `agent/wasi_random/` - os.urandom, random module
- `agent/wasi_clock/` - time.time, monotonic, perf_counter, clock_gettime
- `agent/hash_random/` - Python hash() function, PYTHONHASHSEED
- `agent/environ_args/` - os.environ, sys.argv, sys.path, os.getpid
- `agent/set_order/` - set/frozenset iteration order, dict ordering
- `agent/id_repr/` - id(), repr(), object.__hash__
- `agent/float_math/` - transcendental float math (sin/cos/exp/sqrt/log/atan2)
  + repr(); probe written, NOT verified this session (see TODO: harness blockers)

> NOTE: finding 1 above ("hardcoded seed [GenL,ayer]") is STALE. `random_get` is
> now seeded from `sha3-256(stdin)` (8 LE u32 words) — see preview1.rs Context::new
> and spec 02-wasip1.rst. Determinism is preserved; the seed just varies per input.

## Spec audit (doc/website/src/spec vs executor)

Cross-checked the spec against the implementation. Constants in `constants.rst`
match `sdk-rs/src/abi/consts.rs` (both generated from `codegen/data/public-abi.json`).
Three mismatches found and fixed (committed):
1. `vm_error` codes: spec rendered `OOM:RAM`, wire format is space-joined `OOM RAM`
   (fixed in codegen template `rst.rb`; corroborated by `exit_code 1` in result hashes).
2. `fd_prestat_get`: spec said "Notsup otherwise"; impl returns `Prestat` for dirs,
   `Badf` for everything else.
3. `path_rename` was missing from the WASI Rofs always-erroring list.
