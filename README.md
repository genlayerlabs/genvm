# GenVM
It is a monorepo for GenVM, which consists of the following subprojects:
- [vm](./genvm/) itself: modified [`wasmtime`](https://wasmtime.dev) which exposes genvm-sdk-wasi implementation
- [sdk-rust](./sdk-rust/): rust library with bindings for genvm-sdk-wasi
- [sdk-python](./sdk-python/):
    - slight [RustPython](https://github.com/RustPython/RustPython) modification that removes some floats from core python functionality to prevent crashes
    - bindings for genvm-sdk-wasi:
        - raw [`genlayer.wasi`](./sdk-python/src/lib.rs)
        - user-firendly [`genlayer.sdk](./sdk-python/py/)

## Building

Required tools:
- python3
- ruby (tested with 3.0)
- ninja-build (or just ninja)

Getting the source
1. clone the repositopry
2. `git submodule update --init --recursive`
3. `./tools/git-third-party/git-third-party update --all`
  This command will clone all third-party repositories and then apply patches to them

Actually building became way to complex really fast (patching floats for software ones and so on), for convenience small generator script was introduced
1. create at root a file named `build-config.json` with following contents
    ```json
    {
      "wasiSdk": "/path/to/wasi-sdk/21/sdk-Linux",
      "sdkPython": {
        "debugSdk": true
      }
    }
    ```
2. `./tools/ya-build/ya-build config --config build-config.json`
3. `ninja -C build genvm/sdk-python/all`
