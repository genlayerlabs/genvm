//! Tests for `Decode` on internally tagged enums whose variants share a field
//! *name* but give it different *types* ("ambiguous" fields).
//!
//! Such a field cannot be decoded eagerly — its type is unknown until the tag is
//! seen — so the derived decoder buffers it as a deferred `Maybe<Value>` and only
//! decodes it into the exact per-variant type during assembly (via
//! `Maybe::<Value>::decode_into`). On the byte-backed path that keeps the raw
//! wire bytes and decodes straight into the target type, never building a `Value`.
//!
//! The derive also emits a compile-time (deprecation-style) warning for each
//! ambiguous field; these tests intentionally exercise that path.

use genlayer_calldata::codec::Decode;
use genlayer_calldata::{Decode, Encode, Encoder, Value, codec};

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

/// Decode `bytes` straight from the wire (no intermediate `Value`).
fn from_bytes<T: codec::Decode>(bytes: &[u8]) -> T {
    genlayer_calldata::decode_obj(bytes).unwrap()
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

/// Assert `val` survives both the `Value` path and the direct binary path.
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
    let via_bin: T = from_bytes(&bytes);
    assert_eq!(via_bin, val, "binary-path roundtrip mismatch");
}

// ── Fixtures ─────────────────────────────────────────────────────────

/// The simplest ambiguous case: field `a` is `u32` in one variant, `String` in
/// the other.
#[derive(Debug, PartialEq, Encode, Decode)]
#[calldata(tag = "type")]
enum SameA {
    Foo { a: u32 },
    Bar { a: String },
}

fn de_negate(v: Value) -> Result<i64, codec::DecodeError> {
    let n: i64 = i64::decode(codec::ValueDeserializer(v))?;
    Ok(-n)
}

fn default_payload_u32() -> u32 {
    99
}

/// `id` is monomorphic (`u64` everywhere); `payload` is ambiguous across `u32`,
/// `bool` and an `i64` decoded via `deserialize_with`.
#[derive(Debug, PartialEq, Decode)]
#[calldata(tag = "k")]
enum Mixed {
    Plain {
        id: u64,
        payload: u32,
    },
    Flag {
        id: u64,
        payload: bool,
    },
    Negated {
        id: u64,
        #[calldata(deserialize_with = de_negate)]
        payload: i64,
    },
}

/// An ambiguous field carrying a `default`.
#[derive(Debug, PartialEq, Decode)]
#[calldata(tag = "type")]
enum WithDefaultAmbiguous {
    Num {
        #[calldata(default = default_payload_u32)]
        payload: u32,
    },
    Text {
        payload: String,
    },
}

/// An ambiguous field whose types are nested containers — exercises that the
/// deferred bytes decode straight into a structured target.
#[derive(Debug, PartialEq, Encode, Decode)]
#[calldata(tag = "kind")]
enum Container {
    Ints { items: Vec<u32> },
    Words { items: Vec<String> },
}

// ── User-provided baseline ───────────────────────────────────────────

#[test]
fn same_name_1() {
    let original = SameA::Foo { a: 11 };
    let bytes = to_bytes(&original);
    let got: SameA = genlayer_calldata::decode_obj(&bytes).unwrap();
    assert_eq!(got, original);
}

#[test]
fn same_name_2() {
    let original = SameA::Bar {
        a: "hello".to_string(),
    };
    let bytes = to_bytes(&original);
    let got: SameA = genlayer_calldata::decode_obj(&bytes).unwrap();
    assert_eq!(got, original);
}

// ── Ambiguous scalar roundtrips (both paths) ─────────────────────────

#[test]
fn ambiguous_int_variant_roundtrips() {
    assert_roundtrips(SameA::Foo { a: 7 });
}

#[test]
fn ambiguous_text_variant_roundtrips() {
    assert_roundtrips(SameA::Bar { a: "world".into() });
}

#[test]
fn ambiguous_decodes_from_value_path() {
    let int: SameA = from_value(map([
        ("a", Value::from(123u32)),
        ("type", Value::Str("Foo".into())),
    ]));
    assert_eq!(int, SameA::Foo { a: 123 });

    let text: SameA = from_value(map([
        ("a", Value::Str("hi".into())),
        ("type", Value::Str("Bar".into())),
    ]));
    assert_eq!(text, SameA::Bar { a: "hi".into() });
}

#[test]
fn ambiguous_decodes_from_binary_path() {
    // Encode through `Value` to canonical (sorted) wire bytes, then decode
    // straight into the enum — the deferred field reads from raw bytes.
    let bytes = to_bytes(&map([
        ("a", Value::Str("deferred".into())),
        ("type", Value::Str("Bar".into())),
    ]));
    assert_eq!(
        from_bytes::<SameA>(&bytes),
        SameA::Bar {
            a: "deferred".into()
        }
    );
}

// ── Mixed: monomorphic + ambiguous (+ deserialize_with) ──────────────

#[test]
fn mixed_plain_variant() {
    let val = map([
        ("id", Value::from(1u64)),
        ("k", Value::Str("Plain".into())),
        ("payload", Value::from(10u32)),
    ]);
    assert_eq!(
        from_value::<Mixed>(val),
        Mixed::Plain { id: 1, payload: 10 }
    );
}

#[test]
fn mixed_flag_variant() {
    let val = map([
        ("id", Value::from(2u64)),
        ("k", Value::Str("Flag".into())),
        ("payload", Value::Bool(true)),
    ]);
    assert_eq!(
        from_value::<Mixed>(val),
        Mixed::Flag {
            id: 2,
            payload: true
        }
    );
}

#[test]
fn mixed_negated_variant_uses_deserialize_with() {
    let val = map([
        ("id", Value::from(3u64)),
        ("k", Value::Str("Negated".into())),
        ("payload", Value::from(8i64)),
    ]);
    assert_eq!(
        from_value::<Mixed>(val),
        Mixed::Negated { id: 3, payload: -8 }
    );
}

#[test]
fn mixed_decodes_from_binary_path() {
    let bytes = to_bytes(&map([
        ("id", Value::from(42u64)),
        ("k", Value::Str("Flag".into())),
        ("payload", Value::Bool(false)),
    ]));
    assert_eq!(
        from_bytes::<Mixed>(&bytes),
        Mixed::Flag {
            id: 42,
            payload: false
        }
    );
}

// ── Wrong type for the chosen variant ────────────────────────────────

#[test]
fn ambiguous_wrong_type_for_variant_is_rejected() {
    // Tag says Foo, but the value is a string — decoding it as u32 fails.
    let val = map([
        ("a", Value::Str("not a number".into())),
        ("type", Value::Str("Foo".into())),
    ]);
    assert!(
        try_from_value::<SameA>(val).is_err(),
        "decoding a string as the Foo u32 payload must fail"
    );
}

#[test]
fn ambiguous_wrong_type_for_variant_is_rejected_on_binary_path() {
    let bytes = to_bytes(&map([
        ("a", Value::Str("nope".into())),
        ("type", Value::Str("Foo".into())),
    ]));
    assert!(
        genlayer_calldata::decode_obj::<SameA>(&bytes).is_err(),
        "binary path must also reject the type mismatch"
    );
}

// ── Ambiguous field with a default ───────────────────────────────────

#[test]
fn ambiguous_default_present() {
    let val = map([
        ("payload", Value::from(3u32)),
        ("type", Value::Str("Num".into())),
    ]);
    assert_eq!(
        from_value::<WithDefaultAmbiguous>(val),
        WithDefaultAmbiguous::Num { payload: 3 }
    );
}

#[test]
fn ambiguous_default_missing_uses_default() {
    let val = map([("type", Value::Str("Num".into()))]);
    assert_eq!(
        from_value::<WithDefaultAmbiguous>(val),
        WithDefaultAmbiguous::Num { payload: 99 }
    );
}

#[test]
fn ambiguous_default_other_variant() {
    let val = map([
        ("payload", Value::Str("words".into())),
        ("type", Value::Str("Text".into())),
    ]);
    assert_eq!(
        from_value::<WithDefaultAmbiguous>(val),
        WithDefaultAmbiguous::Text {
            payload: "words".into()
        }
    );
}

// ── Ambiguous container-typed field ──────────────────────────────────

#[test]
fn ambiguous_container_ints_roundtrips() {
    assert_roundtrips(Container::Ints {
        items: vec![1, 2, 3],
    });
}

#[test]
fn ambiguous_container_words_roundtrips() {
    assert_roundtrips(Container::Words {
        items: vec!["a".into(), "b".into()],
    });
}
