# Running GenVM tests

GenVM uses `ya-test-runner` for running all tests. The test runner automatically handles service dependencies (manager, modules, webdriver).

## Prerequisites

- Build the project first: `./configure.rb && ninja -C build`
- For `get_webpage` tests, a compatible webdriver is needed. Use the docker image: `./modules/webdriver/run-test-docker.sh`
- For `exec_prompt` tests, set the `OPENAIKEY` env variable to your OpenAI key

## Running Tests

### Using nix (recommended)
```bash
nix develop .#mock-tests --command ya-test-runner run
```

### Without nix
```bash
# Install ya-test-runner
pip install ./tools/ya-test-runner

# Run all tests
ya-test-runner run

# Run with tag filter
ya-test-runner --filter-tag 'stable' run

# Show available tests
ya-test-runner show test

# Show execution plan
ya-test-runner show plan
```

### Using Presets

Tag expression presets are available in `tests/presets/`:
```bash
# Run release tests (integration & stable)
ya-test-runner --filter-tag "$(cat tests/presets/release.txt)" run

# Run rust tests (rust | integration)
ya-test-runner --filter-tag "$(cat tests/presets/rust.txt)" run

# Run python tests
ya-test-runner --filter-tag "$(cat tests/presets/python.txt)" run
```

### Coverage Collection

To collect coverage for Rust tests:
```bash
nix develop .#rust-test --command ya-test-runner --filter-tag rust --coverage run
```

### Re-running Failed Tests

When tests fail, ya-test-runner automatically writes the failed test names to a continue file at `build/test-artifacts/continue/<timestamp>-<random>`. To re-run only the failed tests:

```bash
# Use the continue file path shown in the failure summary
ya-test-runner --filter-continue 20260123-143052-abc123 run

# Or use a full path
ya-test-runner --filter-continue build/test-artifacts/continue/20260123-143052-abc123 run
```

### Useful Options

- `--filter-name REGEX` - Filter tests by name regex
- `--filter-tag EXPR` - Filter tests by tags (e.g., `stable & !slow`)
- `--filter-continue FILE` - Re-run only tests from a continue file (from a previous failed run)
- `--fail-fast` - Stop on first failure
- `--coverage` - Enable coverage collection for Rust tests
- `--log-level {trace,debug,info,warning,error}` - Set logging level

## Test Categories

- **Integration tests** (`tests/cases/`): End-to-end tests using jsonnet configuration
- **Rust tests** (`executor/tests/`): Unit tests for the Rust executor
- **Python tests** (`runners/genlayer-py-std/test/`): Tests for the Python SDK

## Configuration

The test runner reads configuration from `.ya-test.json` in the project root:

```json
{
    "artifacts_dir": "build/test-artifacts"
}
```

- `artifacts_dir` - Directory for test artifacts (logs, continue files). Defaults to `build/test-artifacts`.
