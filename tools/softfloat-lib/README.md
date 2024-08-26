This tool allows you to remove floats from a wasm file

1. build `softfloat.wasm` and link it before your wasm
    - `python3 build.py <path/to/wasi/sdk>`
2. `patch-floats <in-wasm> <out-wasm>` is the main tool you need
