---
name: test
description: Runs tests for the GenVM project. Use after making code changes to verify correctness.
---

# Running Tests

There are 3 main test suites in GenVM. Before running tests, ensure the project is built (see `/build` skill).

## 1. Python/Poetry Tests

Tests for the Python standard library (`genlayer-py-std`).

```bash
cd runners/genlayer-py-std && poetry install --with dev && poetry run pytest
```

Or with nix:
```bash
nix develop .#py-test --command bash -c 'cd runners/genlayer-py-std && poetry install --with dev && poetry run pytest'
```

**Filter tests:**
```bash
poetry run pytest -k "test_name_pattern"
```

## 2. Rust Tests (`tests/rust.sh`)

Runs cargo tests, AFL fuzzing, and collects coverage for all Rust crates.

```bash
nix develop .#rust-test --command bash tests/rust.sh
```

**Options:**
| Flag | Description |
|------|-------------|
| `--filter REGEX` | Filter which tests/fuzz targets to run |
| `--fuzz-timeout SECS` | Fuzzing duration (default: 30s) |
| `--precompile` | Run precompile for genvm |
| `--update-corpus` | Update fuzz corpus after fuzzing |
| `--no-coverage` | Skip coverage collection |

**Examples:**
```bash
# Run only tests matching "parser"
nix develop .#rust-test --command bash tests/rust.sh --filter 'parser'

# Skip fuzzing, only run unit tests
nix develop .#rust-test --command bash tests/rust.sh --filter '.*/tests'

# Run with longer fuzz timeout
nix develop .#rust-test --command bash tests/rust.sh --fuzz-timeout 120
```

**Prerequisites:** Requires `./configure.rb` to have been run (done by build).

## 3. Integration Tests (`tests/runner/run.py`)

Python-based integration tests that run genvm with test cases.

**Precompile (optional):** If WASM files or WASM compilation process changed, run precompile first to save time:
```bash
./build/out/executor/vTEST/bin/genvm precompile
```

Then run the tests:
```bash
nix develop .#rust-test --command python3 ./tests/runner/run.py --start-manager --start-modules
```

**Options:**
| Flag | Description |
|------|-------------|
| `--filter REGEX` | Filter which test cases to run |
| `--ci` | CI mode |
| `--show-steps` | Show detailed step output |
| `--log-level LEVEL` | Set log level (trace/debug/info/warning/error) |
| `--no-sequential` | Run tests in parallel |
| `--manager URI` | Use existing manager instead of starting one |

**Examples:**
```bash
# Run specific test
nix develop .#rust-test --command python3 ./tests/runner/run.py --start-manager --start-modules --filter 'test_name'

# With debug logging
nix develop .#rust-test --command python3 ./tests/runner/run.py --start-manager --start-modules --log-level debug
```

## Webdriver Setup (for semi-stable/unstable runner tests)

Some tests require a Selenium webdriver. Start it before running those tests:

```bash
bash modules/webdriver/build-and-run.sh
```

This builds and runs a docker container with webdriver on port 4444. The container runs in detached mode.

**Stop webdriver:**
```bash
docker ps | grep 4444 | awk '{print $1}' | xargs docker stop
```

## Quick Reference

| What to test | Command |
|--------------|---------|
| Python stdlib | `cd runners/genlayer-py-std && poetry run pytest` |
| Rust unit tests only | `nix develop .#rust-test --command bash tests/rust.sh --filter '.*/tests' --no-coverage` |
| Precompile (if WASM changed) | `./build/out/executor/vTEST/bin/genvm precompile` |
| Integration tests | `nix develop .#rust-test --command python3 ./tests/runner/run.py --start-manager --start-modules` |
| All rust tests (CI) | `nix develop .#rust-test --command bash tests/rust.sh` |
