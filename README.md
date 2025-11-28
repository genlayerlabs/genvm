# GenVM
[![Twitter](https://img.shields.io/twitter/url/https/twitter.com/yeagerai.svg?style=social&label=Follow%20%40GenLayer)](https://x.com/GenLayer)

GenVM is the execution environment for Intelligent Contracts in the GenLayer protocol. It serves as the backbone for processing and managing contract operations within the GenLayer ecosystem.

GenVM's only purpose is to execute Intelligent Contracts, which can have non-deterministic code while maintaining blockchain security and consistency.

## About

It is a monorepo for GenVM, which consists of the following sub-projects:

- [executor](./executor/) core GenVM itself: modified [`wasmtime`](https://wasmtime.dev) which exposes genvm-sdk-wasi implementation and does all the sandboxing work
- [modules](./modules/): Implementation of modules and manager
- [runners](./runners/): various "runners" available for contracts to use
    - software floating point implementation
    - python interpreter with built-in bindings to genlayer wasm module
    - python standard library from genlayer
    - ...

##  GenVM for users

For "getting started" documentation please refer to [GenLayer documentation](https://docs.genlayer.com/build-with-genlayer/intelligent-contracts)

For more complex examples you can look into [test suite](./tests/cases/)


## Building from source

Required tools:
- git
- ruby (3.\*)
- ninja
- rustup (cargo+rustc)
- (for runners) nix and x86_64 system

All of them (except for the git for obvious reasons) are provided by default shell in `build-scripts/devenv/flake.nix` (for direnv add `use flake ./build-scripts/devenv`)

Prelude:
- `./configure.rb`<br />
  This command scraps and configures all targets (similar to CMake)
- `ninja` is an alternative to `make`, it runs build commands
- Output is located at `build/out` as a "root" (`bin`, `share`)

### Debug build

1. `cd $PROJECT_DIR`
2. `git submodule update --init --recursive --depth 1`
3. `source env.sh` (not needed if you used flake)
4. `git third-party update --all`
5. `./configure.rb`
6. `ninja -C build` (or `ninja -C build all/bin`)
7. Get `genvm-runners.zip` from [github](https://github.com/genlayerlabs/genvm)
8. merge `build/out` and `genvm-runners.zip`

### Production build

WARNING: currently it is supported only on x86_64 linux hosts

1. `cd $PROJECT_DIR`
2. `nix build -o build/out-universal -v -L .#all-for-platform.universal`
3. `nix build -o build/out-amd64-linux -v -L .#all-for-platform.amd64-linux`
4. merge outputs

## Contributing

For contributing documentation see [contributing page](./doc/contributing/README.md)
