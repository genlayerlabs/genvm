# Which python implementation should we use?

1. CPython
2. RustPython

## Problem

GenVM needs to be able to run Python

## Context

## Decision Drivers

1. Deterministic build
2. Maturity
3. Libraries support
4. Performance

## Considered Options

1. CPython
2. RustPython

## Decision Outcome

### Consequences

## Pros and Cons of the Options

CPython advantages:
1. performance (3.6 times faster on `n_queens`, 26 times faster on `json_loads`)
2. is a reference implementation

Rust python advantages:
1. Tier 1 wasm-wasi support
2. Rust bindings are easier to create

Rust python disadvantages:
1. most libraries don't work
