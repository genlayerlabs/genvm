You are security testing a blockchain VM named GenVM. This VM is capable of non-deterministic operations.
You can explore codebase, it is entirely present here

## Strategy

1. Explore which wasm instructions, WASI function calls and so on can lead to non-determinism
2. Write a test suite that potentially exploits it
3. Run it

Please note that LLM and WEB access is "disabled" for the feedback speed (it will return error to non-deterministic block)

use your own `tests/cases/stable/claude/agent/<id>` directory for temporary files

## Permanent files

1. Keep track of your explored paths in `tests/cases/stable/claude/intelligence/EXPLORED_PATHS.md`
2. Pick tasks before starting and dump future tasks before finishing to `tests/cases/stable/claude/intelligence/TODO.md`
3. If you find deterministic violation (hash mismatch) or an internal GenVM error (not mock host one), instantly copy it to `tests/cases/stable/claude/discoveries/<name>/` after that add README.md there with an explanation and stop. Timeout error is not an error. Wasm trap is not an error

## Writing tests

Look at `tests/cases/stable/claude/example/` for a reference test. Each test has:
- One or more `.py` contract files with `# { "Depends": "py-genlayer:test" }` header
- A `.jsonnet` file describing the test scenario (deploy, call methods, etc.)

Templates are in `tests/templates/`. Use `util.jsonnet` (`addPaths`, `chain`) to structure multi-step tests. See `message.json` for the base message format.

For all test transactions specify:
1. `expected_semantics_components: []`
2. `modes: 'lvs'`
3. `stable_hash: false`

For top-level test object specify `tags: ["fuzz"]`

You can write tests in rust, if you find it necessary. See tests/cases/unstable/nondet/web/fetch_webpage_rust for an example

## Running tests

```bash
# from repo root, run a specific test:
./tests/cases/stable/claude/run_test.py <test-dir>

# example:
./tests/cases/stable/claude/run_test.py example

# this runs: ya-test-runner --test-tags integration --test-name 'tests/cases/stable/claude/<test-dir>' run --no-manager
# it assumes a manager is already running on port 3999

# Output lives in directories like
ls build/test-artifacts/integration/stable/claude/example/

# pattern is akin to
# build/test-artifacts/integration/stable/claude/<test-dir>/<jsonnet-name>/{0l,0v,0s}/
```
