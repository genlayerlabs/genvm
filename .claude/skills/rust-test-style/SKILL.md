---
name: rust-test-style
description: GenVM Rust test conventions and style. Use when writing, adding, or modifying Rust tests — inline `#[cfg(test)] mod tests`, integration tests under a crate's `tests/`, helpers, assertions, and how `ya-test-runner` discovers them.
---

# Writing Rust tests in GenVM

Standard library testing only. Do **not** add `rstest`, `proptest`, `insta`, or
`pretty_assertions` — they are not used anywhere in the repo. Plain `#[test]`,
`#[cfg(test)] mod tests`, and `std` assertions are the whole toolkit. `tokio` is
available with the `macros` feature, so use `#[tokio::test]` when a test must be
async (rare; no async tests exist today).

To run these tests, see the `/test` skill (`--filter-tag rust`).

## Where tests go

Two locations, both in use:

1. **Integration tests** — one file per concern under the crate's `tests/` dir.
   Preferred for exercising a crate's public API. Each `tests/*.rs` file is its
   own compilation unit and gets its own runner case.
   - e.g. `executor/crates/calldata/tests/derive_decode.rs`,
     `executor/crates/common/tests/expr.rs`,
     `modules/implementation/tests/test_rat.rs`
2. **Inline unit tests** — `#[cfg(test)] mod tests { ... }` at the bottom of a
   `src/*.rs` file, for testing private items. e.g.
   `executor/crates/calldata/src/lib.rs`, `executor/crates/common/src/logger/mod.rs`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_value_in_struct() { /* ... */ }
}
```

## Helpers over fixtures

There is no fixture framework. Factor repeated setup into small free functions
at the **top of the test file**, then keep each `#[test]` short. This is the
dominant pattern.

```rust
// executor/crates/common/tests/expr.rs:5
fn eval(input: &str) -> Value {
    Expr::parse(input).unwrap().evaluate().unwrap()
}

fn assert_rational(val: Value, n: i64, d: i64) {
    let r = val.into_rational().unwrap();
    assert_eq!(r, BigRational::new(BigInt::from(n), BigInt::from(d)));
}
```

```rust
// modules/implementation/tests/test_rat.rs — construct the system under test
fn make_vm() -> Lua {
    let vm = Lua::new();
    rat::register_rat_global(&vm).unwrap();
    vm
}
fn eval<T: mlua::FromLua>(vm: &Lua, code: &str) -> T {
    vm.load(code).eval::<T>().unwrap()
}
```

Define throwaway `#[derive(...)]` types as fixtures right in the test file:

```rust
// executor/crates/calldata/tests/derive_decode_excess_fields.rs:11
#[derive(Debug, PartialEq, Decode)]
struct Named { x: i32, y: String }
```

## Assertions

`std` only: `assert!`, `assert_eq!`, `assert_ne!`. When asserting a boolean
condition, add a format-string message that prints the **actual value** so a
failure is debuggable:

```rust
let msg = err.to_string();
assert!(msg.contains("unknown field"), "unexpected error: {msg}");
assert!(msg.contains("`z`"), "should mention field name: {msg}");
```

For value equality use `assert_eq!` against a fully-constructed expected value
(derive `Debug, PartialEq` on the type).

## Testing errors

Use a helper that returns `Result`, call `.unwrap_err()`, and assert on
substrings of the `Display` message — don't match on error enum internals:

```rust
// executor/crates/calldata/tests/derive_decode_excess_fields.rs:5
fn try_decode_from_value<T: codec::Decode>(val: Value) -> Result<T, codec::DecodeError> {
    T::decode(codec::ValueDeserializer(val))
}

#[test]
fn tuple_struct_rejects_too_many_elements() {
    let err = try_decode_from_value::<Tuple>(/* 3 elems */).unwrap_err();
    assert!(err.to_string().contains("expected 2"), "unexpected error: {err}");
}
```

The happy path is liberal with `.unwrap()` — a panic *is* the test failure.

## Roundtrip pattern

For codecs, encode → decode → compare in one helper:

```rust
// executor/crates/calldata/tests/derive_decode.rs:5
fn roundtrip_via_value<T>(val: &T) -> Value
where T: for<'a> codec::Encode<&'a mut Vec<u8>, Error = std::convert::Infallible> {
    let mut buf = Vec::new();
    codec::Encode::encode(val, &mut Encoder::new(&mut buf)).unwrap();
    genlayer_calldata::decode(&buf).unwrap()
}
```

## Naming & layout

- Test fn names are descriptive `snake_case` that name the **scenario**, e.g.
  `named_struct_rejects_unknown_field`, `string_literals_and_escapes`.
- **Do not prefix with `test_`.** The `#[test]` attribute already says it is a
  test; the prefix is noise. Drop it from new tests and from any you touch.
- Group related tests with a section-comment banner:
  ```rust
  // ── Tuple struct: wrong sequence length ─────────────────────────────
  ```
- Helpers first, then types, then `#[test]` functions.

## How `ya-test-runner` discovers Rust tests

Discovery lives in `tests/runner/ya_test_runner_plugins/cargo.py`. For each
registered Rust crate root it creates cases via plain `cargo test`:

| Source | Command | Tags |
|--------|---------|------|
| each `tests/*.rs` file | `cargo test --test <file>` | `rust`, `unit` |
| crate has `src/lib.rs` (inline `#[cfg(test)]`) | `cargo test --lib` | `rust`, `unit` |
| each binary (`src/main.rs` / `[[bin]]`) | `cargo test --bin <name>` | `rust`, `unit` |
| each `[[example]]` | `cargo check --example <name>` (compile-only) | `rust`, `example` |

Consequences when adding tests:

- **A new file in `tests/` is auto-discovered** — no registration needed, as long
  as the crate root itself is already scanned.
- **New inline `#[cfg(test)]` tests** ride along on the existing `--lib` case;
  nothing to register.
- The crate must be a known Rust root. Roots carry a `.ya-test-config.json`
  (e.g. `executor/.ya-test-config.json`, `executor/crates/calldata/.ya-test-config.json`).
  That file controls per-crate `cargo_test_flags`, `keep_env`, and a `skip` map
  (skip `examples`/specific cases) — add a new crate's config there if introducing
  a new root.
- The `rust` preset is `(rust | integration) & !bench & !fuzz`
  (`tests/presets/rust.txt`).

## Prefer fuzzing when it fits

If a function's correctness is really about holding over a **space of inputs**
(parsers, codecs, encode/decode roundtrips, anything taking arbitrary bytes or
structured input), write a **fuzz target** rather than a handful of hand-picked
`#[test]` cases. A few cherry-picked examples give false confidence; a fuzzer
explores the space. Don't settle for an under-tested `#[test]` when the property
is fuzz-shaped — reach for fuzz, or do both (fuzz for coverage, a couple of
`#[test]`s pinning known edge cases / regressions).

Fuzz targets live under a crate's `fuzz/` (afl, behind the `arbitrary` feature,
tagged `fuzz`, built with `cargo_afl_build_flags`; see `cargo_fuzz` in
`tests/runner/ya_test_runner_plugins/cargo.py`). They are a distinct case type —
don't mix a fuzz harness into `tests/` or `mod tests`, and they're excluded from
the `rust` preset (`!fuzz`).
