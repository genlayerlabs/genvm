//! Tests for `Decode` derivation on internally tagged enums
//! (`#[calldata(tag = "...")]`).
//!
//! The derived decoder streams the map in a single pass — no intermediate
//! `Value` is built — so these tests exercise both the in-memory `Value` path
//! (`ValueDeserializer`) and the direct binary path (`decode_obj`), and pin down
//! the error behaviour: length-range gate, unknown fields, missing tag/fields,
//! unknown variants and tag-position independence (the tag may be sorted before,
//! between or after a variant's fields).

use genlayer_calldata::codec::Decode;
use genlayer_calldata::{Decode, Encode, Encoder, Value, codec};
use std::collections::BTreeMap;

// ── Helpers ──────────────────────────────────────────────────────────

/// Encode a value to the binary wire format.
fn to_bytes<T>(val: &T) -> Vec<u8>
where
    T: for<'a> codec::Encode<&'a mut Vec<u8>, Error = std::convert::Infallible>,
{
    let mut buf = Vec::new();
    codec::Encode::encode(val, &mut Encoder::new(&mut buf)).unwrap();
    buf
}

/// Decode a `Value` into the target type via `ValueDeserializer`.
fn from_value<T: codec::Decode>(val: Value) -> T {
    T::decode(codec::ValueDeserializer(val)).unwrap()
}

fn try_from_value<T: codec::Decode>(val: Value) -> Result<T, codec::DecodeError> {
    T::decode(codec::ValueDeserializer(val))
}

/// Full roundtrip: assert the value survives both the `Value` path and the
/// direct binary path (`decode_obj`, which never materializes a `Value`).
fn assert_roundtrips<T>(val: T)
where
    T: PartialEq
        + std::fmt::Debug
        + codec::Decode
        + for<'a> codec::Encode<&'a mut Vec<u8>, Error = std::convert::Infallible>,
{
    let bytes = to_bytes(&val);

    let via_value: T = from_value(genlayer_calldata::decode(&bytes).unwrap());
    assert_eq!(via_value, val, "value-path roundtrip mismatch");

    let via_bin: T = genlayer_calldata::decode_obj(&bytes).unwrap();
    assert_eq!(via_bin, val, "binary-path roundtrip mismatch");
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

// ── Fixtures ─────────────────────────────────────────────────────────

/// Mixed variant kinds, and field names chosen so the tag (`"type"`) lands
/// before, between and after the variant's fields in sorted key order.
#[derive(Debug, PartialEq, Encode, Decode)]
#[calldata(tag = "type")]
enum Shape {
    // {"type": "Empty"}
    Empty,
    // sorted keys: radius, type   → tag last
    Circle {
        radius: u32,
    },
    // sorted keys: height, type, width → tag in the middle
    Rect {
        height: u32,
        width: u32,
    },
    // sorted keys: alpha, type, z_last → tag in the middle, plus renames
    #[calldata(rename = "named_shape")]
    Named {
        alpha: i32,
        #[calldata(rename = "z_last")]
        last: String,
    },
    // sorted keys: type, val → tag first
    Renamed {
        val: i64,
    },
}

/// Two variants sharing a field of the same name and type.
#[derive(Debug, PartialEq, Encode, Decode)]
#[calldata(tag = "t")]
enum Shared {
    X { id: u64, x_only: bool },
    Y { id: u64, y_only: bool },
}

fn default_count() -> u32 {
    7
}

#[derive(Debug, PartialEq, Decode)]
#[calldata(tag = "kind")]
enum WithDefault {
    A {
        name: String,
        #[calldata(default = default_count)]
        count: u32,
    },
    B,
}

fn de_doubled(v: Value) -> Result<u32, codec::DecodeError> {
    let n: u32 = u32::decode(codec::ValueDeserializer(v))?;
    Ok(n * 2)
}

#[derive(Debug, PartialEq, Decode)]
#[calldata(tag = "type")]
enum WithDeserializeWith {
    Scaled {
        #[calldata(deserialize_with = de_doubled)]
        value: u32,
    },
}

// ── Happy-path roundtrips ────────────────────────────────────────────

#[test]
fn unit_variant_roundtrips() {
    assert_roundtrips(Shape::Empty);
}

#[test]
fn named_variant_tag_last_roundtrips() {
    assert_roundtrips(Shape::Circle { radius: 42 });
}

#[test]
fn named_variant_tag_in_middle_roundtrips() {
    assert_roundtrips(Shape::Rect {
        height: 3,
        width: 9,
    });
}

#[test]
fn named_variant_with_renames_roundtrips() {
    assert_roundtrips(Shape::Named {
        alpha: -5,
        last: "tail".into(),
    });
}

#[test]
fn renamed_variant_tag_first_roundtrips() {
    assert_roundtrips(Shape::Renamed { val: -123456789 });
}

#[test]
fn shared_field_variants_roundtrip() {
    assert_roundtrips(Shared::X {
        id: 1,
        x_only: true,
    });
    assert_roundtrips(Shared::Y {
        id: 2,
        y_only: false,
    });
}

// ── Tag-position independence (explicit key orders) ──────────────────

#[test]
fn decodes_with_tag_in_the_middle() {
    // height < type < width
    let val = map([
        ("height", Value::from(1u32)),
        ("type", Value::Str("Rect".into())),
        ("width", Value::from(2u32)),
    ]);
    let got: Shape = from_value(val);
    assert_eq!(
        got,
        Shape::Rect {
            height: 1,
            width: 2
        }
    );
}

#[test]
fn decodes_with_tag_last() {
    // radius < type
    let val = map([
        ("radius", Value::from(8u32)),
        ("type", Value::Str("Circle".into())),
    ]);
    let got: Shape = from_value(val);
    assert_eq!(got, Shape::Circle { radius: 8 });
}

// ── Defaults ─────────────────────────────────────────────────────────

#[test]
fn default_field_present() {
    let val = map([
        ("kind", Value::Str("A".into())),
        ("name", Value::Str("alice".into())),
        ("count", Value::from(3u32)),
    ]);
    let got: WithDefault = from_value(val);
    assert_eq!(
        got,
        WithDefault::A {
            name: "alice".into(),
            count: 3,
        }
    );
}

#[test]
fn default_field_missing_uses_default() {
    let val = map([
        ("kind", Value::Str("A".into())),
        ("name", Value::Str("bob".into())),
    ]);
    let got: WithDefault = from_value(val);
    assert_eq!(
        got,
        WithDefault::A {
            name: "bob".into(),
            count: 7,
        }
    );
}

// ── deserialize_with ─────────────────────────────────────────────────

#[test]
fn deserialize_with_is_applied() {
    let val = map([
        ("type", Value::Str("Scaled".into())),
        ("value", Value::from(21u32)),
    ]);
    let got: WithDeserializeWith = from_value(val);
    assert_eq!(got, WithDeserializeWith::Scaled { value: 42 });
}

// ── Missing tag ──────────────────────────────────────────────────────

#[test]
fn missing_tag_is_field_missing() {
    let val = map([("radius", Value::from(1u32))]);
    let err = try_from_value::<Shape>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("field missing"), "unexpected error: {msg}");
    assert!(msg.contains("type"), "should mention tag field: {msg}");
}

// ── Missing required field ───────────────────────────────────────────

#[test]
fn missing_required_field_is_field_missing() {
    // Rect needs both height and width; provide only height.
    let val = map([
        ("height", Value::from(1u32)),
        ("type", Value::Str("Rect".into())),
    ]);
    let err = try_from_value::<Shape>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("field missing"), "unexpected error: {msg}");
    assert!(msg.contains("width"), "should mention missing field: {msg}");
}

// ── Unknown variant tag ──────────────────────────────────────────────

#[test]
fn unknown_variant_tag_is_rejected() {
    let val = map([("type", Value::Str("Nope".into()))]);
    let err = try_from_value::<Shape>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown variant"), "unexpected error: {msg}");
    assert!(msg.contains("Nope"), "should mention the bad tag: {msg}");
}

// ── Unknown field (within the length range) ──────────────────────────

#[test]
fn unknown_field_within_range_is_rejected() {
    // Circle is {radius, type} (len 2) which is inside [1, 3]; "q" is unknown.
    let val = map([
        ("q", Value::from(0u32)),
        ("type", Value::Str("Circle".into())),
    ]);
    let err = try_from_value::<Shape>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field"), "unexpected error: {msg}");
    assert!(msg.contains("`q`"), "should mention field name: {msg}");
}

// ── Foreign field (belongs to another variant) ───────────────────────

#[test]
fn field_of_another_variant_is_rejected() {
    // Decoding Y, but x_only belongs only to X. len 3 == max, so it passes the
    // length gate and must be rejected as an unknown field for this variant.
    let val = map([
        ("id", Value::from(1u64)),
        ("t", Value::Str("Y".into())),
        ("x_only", Value::Bool(true)),
    ]);
    let err = try_from_value::<Shared>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field"), "unexpected error: {msg}");
    assert!(msg.contains("`x_only`"), "should mention field name: {msg}");
}

// ── Length range gate ────────────────────────────────────────────────

#[test]
fn too_many_fields_is_length_mismatch() {
    // Shape's widest variant has 3 entries (2 fields + tag); 4 is out of range.
    let val = map([
        ("a", Value::from(0u32)),
        ("b", Value::from(0u32)),
        ("radius", Value::from(1u32)),
        ("type", Value::Str("Circle".into())),
    ]);
    let err = try_from_value::<Shape>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected 3"), "unexpected error: {msg}");
    assert!(msg.contains("got 4"), "unexpected error: {msg}");
}

#[test]
fn empty_map_is_length_mismatch() {
    let val = Value::Map(BTreeMap::new());
    let err = try_from_value::<Shape>(val).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("expected 1"), "unexpected error: {msg}");
    assert!(msg.contains("got 0"), "unexpected error: {msg}");
}

// ── Tag value of the wrong type ──────────────────────────────────────

#[test]
fn non_string_tag_is_rejected() {
    // The tag value is decoded straight as a string; a number is a type error.
    let val = map([("type", Value::from(5u32))]);
    assert!(
        try_from_value::<Shape>(val).is_err(),
        "non-string tag must not decode"
    );
}

// ── Binary-path error parity ─────────────────────────────────────────

#[test]
fn binary_path_rejects_unknown_field() {
    // Build a valid Circle, then re-encode with an extra in-range unknown key by
    // going through Value (keeps things in the same sorted wire form).
    let val = map([
        ("q", Value::from(0u32)),
        ("type", Value::Str("Circle".into())),
    ]);
    // Round-trip the Value to bytes via the generic encoder, then decode_obj.
    let bytes = to_bytes(&val);
    let err = genlayer_calldata::decode_obj::<Shape>(&bytes).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field"), "unexpected error: {msg}");
    assert!(msg.contains("`q`"), "should mention field name: {msg}");
}

#[test]
fn binary_path_decodes_tag_in_middle() {
    let original = Shape::Rect {
        height: 11,
        width: 22,
    };
    let bytes = to_bytes(&original);
    let got: Shape = genlayer_calldata::decode_obj(&bytes).unwrap();
    assert_eq!(got, original);
}
