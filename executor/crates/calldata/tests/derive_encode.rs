use std::collections::BTreeMap;

use genlayer_calldata::{Encode, Encoder, Value, codec};

fn encode_and_decode<T>(val: &T) -> Value
where
    T: for<'a> codec::Encode<&'a mut Vec<u8>, Error = std::convert::Infallible>,
{
    let mut buf = Vec::new();
    codec::Encode::encode(val, &mut Encoder::new(&mut buf)).unwrap();
    genlayer_calldata::decode(&buf).unwrap()
}

#[derive(Encode)]
struct EncStruct {
    b: String,
    a: u32,
}

#[derive(Encode)]
struct EncNewtype(u64);

#[derive(Encode)]
struct EncUnit;

#[derive(Encode)]
enum EncEnum {
    Foo,
    #[calldata(rename = "bar_renamed")]
    Bar(u32),
    Baz {
        x: i32,
        y: String,
    },
}

#[derive(Encode)]
#[calldata(tag = "type")]
enum EncTagged {
    Alpha,
    Beta { val: bool },
}

#[test]
fn struct_named_fields() {
    let s = EncStruct {
        b: "hello".into(),
        a: 42,
    };
    let decoded = encode_and_decode(&s);
    let expected = Value::Map(BTreeMap::from([
        ("a".into(), Value::from(42u32)),
        ("b".into(), Value::from("hello")),
    ]));
    assert_eq!(decoded, expected);
}

#[test]
fn newtype() {
    assert_eq!(encode_and_decode(&EncNewtype(7)), Value::from(7u64));
}

#[test]
fn unit_struct() {
    assert_eq!(encode_and_decode(&EncUnit), Value::Null);
}

#[test]
fn enum_unit_variant() {
    assert_eq!(encode_and_decode(&EncEnum::Foo), Value::Str("Foo".into()),);
}

#[test]
fn enum_newtype_renamed() {
    let decoded = encode_and_decode(&EncEnum::Bar(99));
    let expected = Value::Map(BTreeMap::from([("bar_renamed".into(), Value::from(99u64))]));
    assert_eq!(decoded, expected);
}

#[test]
fn enum_struct_variant() {
    let decoded = encode_and_decode(&EncEnum::Baz {
        x: -1,
        y: "hi".into(),
    });
    let inner = Value::Map(BTreeMap::from([
        ("x".into(), Value::from(-1i64)),
        ("y".into(), Value::from("hi")),
    ]));
    let expected = Value::Map(BTreeMap::from([("Baz".into(), inner)]));
    assert_eq!(decoded, expected);
}

#[test]
fn tagged_unit_variant() {
    let decoded = encode_and_decode(&EncTagged::Alpha);
    let expected = Value::Map(BTreeMap::from([(
        "type".into(),
        Value::Str("Alpha".into()),
    )]));
    assert_eq!(decoded, expected);
}

#[test]
fn tagged_struct_variant() {
    let decoded = encode_and_decode(&EncTagged::Beta { val: true });
    let expected = Value::Map(BTreeMap::from([
        ("type".into(), Value::Str("Beta".into())),
        ("val".into(), Value::Bool(true)),
    ]));
    assert_eq!(decoded, expected);
}
