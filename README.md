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

Download WASI sdk
```bash
mkdir -p build
cd build
wget https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz
# ^ note that your link may be different
tar -xvf wasi-sdk-24.0-x86_64-linux.tar.gz
mv wasi-sdk-24.0-x86_64-linux wasi-sdk-24
```

Actually building became way too complex really fast (patching floats for software ones and so on), for convenience small generator script was introduced
1. `./tools/ya-build/ya-build config`
2. `ninja -C build`
