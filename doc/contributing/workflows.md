# Common workflows

## Adding new LLM provider
**IMPORTANT**: If your provider is compatible with openai API no additional work is needed

- go to [`executor/modules/implementation/src/llm/providers.rs`](../../executor/modules/implementation/src/llm/providers.rs)
- declare and implement new provider (`impl Provider`). It is recommended to implement separate json mode
- add new value to `Provider` enum in `executor/modules/implementation/src/llm/config.rs`
- add case to `BackendConfig::to_provider` in `executor/modules/implementation/src/llm/config.rs`

### Adding test
(optional)

- add test case to [`executor/modules/implementation/src/llm/handler.rs`](../../executor/modules/implementation/src/llm/handler.rs)
- patch [workflow](../../.github/workflows/module-test-cargo.yaml) to pass secret
- provide api key to repository owners

## Adding new wasm function to gl_call
It is an easier approach than next. Just add definition to `executor/src/wasi/gl_call.rs`. You must also add version check to implementation in `executor/src/wasi/genlayer_sdk.rs`.

## Adding new wasm function
- `executor/src/wasi/witx/genlayer_sdk.witx`<br>
    add declaration here
- `executor/src/wasi/genlayer_sdk.rs`<br>
    add implementation here (under `impl` trait)
- `runners/cpython/modules/_genlayer_wasi/genlayer.c`<br>
    add python proxy<br>
    NOTE: this will change hash, rebuilding will show you the new one

## Adding new host function

- `executor/codegen/data/host-fns.json`<br>
    add new function id<br>
    after rebuilding (`tags/codegen`) few files will be updated:
    - `executor/src/host/host_fns.rs`
    - `executor/testdata/runner/host_fns.py`
- `executor/testdata/runner/base_host.py`<br>
    update `while True` to handle new case, add new method to the `IHost` protocol<br>
    NOTE: this file is used in simulator as well (under `backend/node/genvm/origin/`)
- `executor/testdata/runner/mock_host.py`<br>
    add implementation for tests
- update simulator and node
