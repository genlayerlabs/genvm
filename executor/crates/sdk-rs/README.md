# GenLayer SDK for Rust

This crate provides the SDK for building GenLayer intelligent contracts in Rust,
compiled to WebAssembly (wasm32-wasip1 target).

## Building

Contracts must be compiled for the wasm32-wasip1 target:

```bash
cargo build --target wasm32-wasip1 --release
```

## Example

Build the example contract:

```bash
cargo build --example fetch_webpage --target wasm32-wasip1
```

## Important Notes

Due to reference type being disabled, one may need to "normalize" wasm indirect calls used for function pointer invocation

Floating point operations are forbidden in deterministic mode. Invoking them will result in a VM error
