---
name: test
description: Runs tests for the GenVM project. Use after making code changes to verify correctness.
---

# Running Tests

GenVM uses `ya-test-runner` for all tests. Before running tests, ensure the project is built (see `/build` skill).

## Quick Start

Run all tests:
```bash
nix develop .#mock-tests --command ya-test-runner run
```

Run release tests (stable integration):
```bash
nix develop .#mock-tests --command ya-test-runner --filter-tag "$(cat tests/presets/release.txt)" run
```

Run a specific test:
```bash
nix develop .#mock-tests --command ya-test-runner run --filter-name 'test_name'
```

## ya-test-runner Commands

### Run Tests
```bash
ya-test-runner run [OPTIONS]
```

**Options:**
| Flag | Description |
|------|-------------|
| `--filter-name REGEX` | Filter tests by name regex |
| `--filter-tag EXPR` | Filter tests by tags (e.g., `stable & !slow`) |
| `--filter-continue FILE` | Re-run only tests from a continue file |
| `--fail-fast` | Stop execution after first failure |
| `--coverage` | Enable coverage collection for Rust tests |
| `--log-level LEVEL` | Set log level (trace/debug/info/warning/error) |

### Show Information
```bash
# Show available tests
ya-test-runner show test

# Show execution plan
ya-test-runner show plan

# Show available services
ya-test-runner show services

# Show available tags
ya-test-runner show tags
```

## Test Presets

Presets are tag expressions stored in `tests/presets/`:

| Preset | Expression | Use Case |
|--------|------------|----------|
| `release.txt` | `integration & stable` | CI release tests |
| `rust.txt` | `rust \| integration` | Rust development |
| `python.txt` | `python` | Python SDK tests |

Usage:
```bash
ya-test-runner --filter-tag "$(cat tests/presets/release.txt)" run
```

## Test Categories

### Integration Tests (`tests/cases/`)
End-to-end tests using jsonnet configuration. Services (manager, modules, webdriver) are started automatically.

```bash
nix develop .#mock-tests --command ya-test-runner --filter-tag integration run
```

### Rust Tests
Cargo tests for Rust crates:
```bash
nix develop .#rust-test --command ya-test-runner --filter-tag rust run
```

With coverage:
```bash
nix develop .#rust-test --command ya-test-runner --filter-tag rust --coverage run
```

### Python Tests
Tests for the Python standard library (`genlayer-py-std`):
```bash
nix develop .#mock-tests --command ya-test-runner --filter-tag python run
```

Or directly with pytest inside nix develop:
```bash
nix develop .#mock-tests --command bash -c "cd runners/genlayer-py-std && poetry run pytest tests/"
```

**Coverage:** pytest is configured with `--cov` and `--cov-fail-under=75`. Coverage scope includes `genlayer.types`, `genlayer.calldata`, `genlayer.storage`, `genlayer.evm`, `genlayer._internal`, and `genlayer_embeddings`. Must run inside nix develop for numpy-dependent tests and correct coverage resolution.

## Webdriver Setup

For web-related tests (semi-stable/unstable), webdriver is started automatically by ya-test-runner. To manually start it:

```bash
bash modules/webdriver/build-and-run.sh
```

## Precompile (Optional)

If WASM files or compilation changed, precompile to save test time:
```bash
./build/out/bin/genvm precompile
```

## Re-running Failed Tests

When tests fail, ya-test-runner writes failed test names to `build/test-artifacts/continue/<timestamp>-<random>`. Re-run only failed tests:

```bash
# Use filename shown in failure summary
ya-test-runner --filter-continue 20260123-143052-abc123 run
```

## Quick Reference

| What to test | Command |
|--------------|---------|
| All tests | `nix develop .#mock-tests --command ya-test-runner run` |
| Release tests | `nix develop .#mock-tests --command ya-test-runner --filter-tag "$(cat tests/presets/release.txt)" run` |
| Rust tests | `nix develop .#rust-test --command ya-test-runner --filter-tag rust run` |
| Python (poetry) | `nix develop .#mock-tests --command bash -c "cd runners/genlayer-py-std && poetry run pytest tests/"` |
| Re-run failed | `ya-test-runner --filter-continue <file> run` |
| With debug logs | `nix develop .#mock-tests --command ya-test-runner run --log-level debug` |
| Show test list | `nix develop .#mock-tests --command ya-test-runner show test` |
