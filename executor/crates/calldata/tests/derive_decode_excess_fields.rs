use std::collections::BTreeMap;

use genlayer_calldata::{Decode, Value, codec};

fn try_decode_from_value<T: codec::Decode>(val: Value) -> Result<T, codec::DecodeError> {
    T::decode(codec::ValueDeserializer(val))
}

// ── Types ───────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Decode)]
struct Named {
    x: i32,
    y: String,
}

#[derive(Debug, PartialEq, Decode)]
struct Tuple(u32, u32);

#[derive(Debug, PartialEq, Decode)]
enum External {
    Unit,
    Wrap(u32),
    Struct { a: i32 },
}

#[derive(Debug, PartialEq, Decode)]
#[calldata(tag = "t")]
enum Tagged {
    A,
    B { v: bool },
}

// ── Struct: unknown map key ─────────────────────────────────────────

#[test]
fn named_struct_rejects_unknown_field() {
    let val = Value::Map(BTreeMap::from([
        ("x".into(), Value::from(1i32)),
        ("y".into(), Value::Str("hi".into())),
        ("z".into(), Value::from(99i32)),
    ]));
    let err = try_decode_from_value::<Named>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field"), "unexpected error: {msg}");
    assert!(msg.contains("`z`"), "should mention field name: {msg}");
}

// ── Tuple struct: wrong sequence length ─────────────────────────────

#[test]
fn tuple_struct_rejects_too_many_elements() {
    let val = Value::Array(vec![
        Value::from(1u32),
        Value::from(2u32),
        Value::from(3u32),
    ]);
    let err = try_decode_from_value::<Tuple>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected 2"), "unexpected error: {msg}");
}

#[test]
fn tuple_struct_rejects_too_few_elements() {
    let val = Value::Array(vec![Value::from(1u32)]);
    let err = try_decode_from_value::<Tuple>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected 2"), "unexpected error: {msg}");
}

// ── External enum struct variant: unknown field ─────────────────────

#[test]
fn external_enum_struct_variant_rejects_unknown_field() {
    let val = Value::Map(BTreeMap::from([(
        "Struct".into(),
        Value::Map(BTreeMap::from([
            ("a".into(), Value::from(1i32)),
            ("extra".into(), Value::from(2i32)),
        ])),
    )]));
    let err = try_decode_from_value::<External>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field"), "unexpected error: {msg}");
    assert!(msg.contains("`extra`"), "should mention field name: {msg}");
}

// ── Tagged enum: unknown field ──────────────────────────────────────

#[test]
fn tagged_enum_rejects_unknown_field() {
    let val = Value::Map(BTreeMap::from([
        ("t".into(), Value::Str("B".into())),
        ("v".into(), Value::Bool(true)),
        ("extra".into(), Value::from(0i32)),
    ]));
    let err = try_decode_from_value::<Tagged>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field"), "unexpected error: {msg}");
    assert!(msg.contains("`extra`"), "should mention field name: {msg}");
}

// ── Valid inputs still work ─────────────────────────────────────────

#[test]
fn named_struct_exact_fields_ok() {
    let val = Value::Map(BTreeMap::from([
        ("x".into(), Value::from(5i32)),
        ("y".into(), Value::Str("ok".into())),
    ]));
    let result = try_decode_from_value::<Named>(val).unwrap();
    assert_eq!(
        result,
        Named {
            x: 5,
            y: "ok".into()
        }
    );
}

#[test]
fn tuple_struct_exact_length_ok() {
    let val = Value::Array(vec![Value::from(10u32), Value::from(20u32)]);
    let result = try_decode_from_value::<Tuple>(val).unwrap();
    assert_eq!(result, Tuple(10, 20));
}
