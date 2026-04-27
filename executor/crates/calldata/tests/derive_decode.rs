use std::collections::BTreeMap;

use genlayer_calldata::{Decode, Encode, Encoder, Value, codec};

/// Encode a value, then decode the binary back to Value, then decode that Value
/// into the target type via ValueDeserializer.
fn roundtrip_via_value<T>(val: &T) -> Value
where
    T: for<'a> codec::Encode<&'a mut Vec<u8>, Error = std::convert::Infallible>,
{
    let mut buf = Vec::new();
    codec::Encode::encode(val, &mut Encoder::new(&mut buf)).unwrap();
    genlayer_calldata::decode(&buf).unwrap()
}

fn decode_from_value<T: codec::Decode>(val: Value) -> T {
    T::decode(codec::ValueDeserializer(val)).unwrap()
}

// ── Struct types ─────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Encode, Decode)]
struct Simple {
    b: String,
    a: u32,
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Newtype(u64);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Unit;

// ── Enum types ───────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Encode, Decode)]
enum External {
    Foo,
    #[calldata(rename = "bar_renamed")]
    Bar(u32),
    Baz {
        x: i32,
        y: String,
    },
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[calldata(tag = "type")]
enum Tagged {
    Alpha,
    Beta { val: bool },
}

// ── Struct with default ──────────────────────────────────────────────

fn default_score() -> u32 {
    42
}

#[derive(Debug, PartialEq, Decode)]
struct WithDefault {
    name: String,
    #[calldata(default = default_score)]
    score: u32,
}

// ── Tests ────────────────────────────────────────────────────────────

#[test]
fn struct_roundtrip() {
    let orig = Simple {
        b: "hello".into(),
        a: 7,
    };
    let val = roundtrip_via_value(&orig);
    let back: Simple = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn newtype_roundtrip() {
    let orig = Newtype(99);
    let val = roundtrip_via_value(&orig);
    let back: Newtype = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn unit_roundtrip() {
    let orig = Unit;
    let val = roundtrip_via_value(&orig);
    let back: Unit = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn enum_unit_roundtrip() {
    let orig = External::Foo;
    let val = roundtrip_via_value(&orig);
    let back: External = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn enum_newtype_renamed_roundtrip() {
    let orig = External::Bar(55);
    let val = roundtrip_via_value(&orig);
    let back: External = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn enum_struct_variant_roundtrip() {
    let orig = External::Baz {
        x: -3,
        y: "world".into(),
    };
    let val = roundtrip_via_value(&orig);
    let back: External = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn tagged_unit_roundtrip() {
    let orig = Tagged::Alpha;
    let val = roundtrip_via_value(&orig);
    let back: Tagged = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn tagged_struct_roundtrip() {
    let orig = Tagged::Beta { val: true };
    let val = roundtrip_via_value(&orig);
    let back: Tagged = decode_from_value(val);
    assert_eq!(back, orig);
}

#[test]
fn default_field_present() {
    let val = Value::Map(BTreeMap::from([
        ("name".into(), Value::Str("alice".into())),
        ("score".into(), Value::from(10u32)),
    ]));
    let result: WithDefault = decode_from_value(val);
    assert_eq!(
        result,
        WithDefault {
            name: "alice".into(),
            score: 10
        }
    );
}

#[test]
fn default_field_missing() {
    let val = Value::Map(BTreeMap::from([("name".into(), Value::Str("bob".into()))]));
    let result: WithDefault = decode_from_value(val);
    assert_eq!(
        result,
        WithDefault {
            name: "bob".into(),
            score: 42
        }
    );
}
