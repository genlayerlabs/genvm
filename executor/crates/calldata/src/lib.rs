extern crate self as genlayer_calldata;

mod bin;
pub mod codec;
pub mod consts;
mod encoder;
mod error;
mod types;
pub mod unparsed;

pub use encoder::{CounterWriter, Encoder, StdWriter, Writer};
pub use genlayer_calldata_derive::*;

pub use bin::{BinDecodeError, Options as DecodeOptions, decode, decode_with, encode, encode_to};
pub use error::*;
pub use types::*;

pub fn from_value<T>(value: Value) -> core::result::Result<T, codec::DecodeError>
where
    T: codec::Decode,
{
    T::decode(codec::ValueDeserializer(value))
}

pub fn to_value(value: &impl codec::Encode<Vec<u8>, Error = std::convert::Infallible>) -> Value {
    let buf = Vec::new();
    let mut enc = Encoder::new(buf);
    match value.encode(&mut enc) {
        Ok(()) => {}
        Err(e) => match e {},
    }
    let buf = enc.into_inner();
    decode(&buf).expect("encode-decode roundtrip failed")
}

/// Encode a value directly to bytes, skipping the intermediate `Value` representation.
pub fn encode_obj(
    value: &impl codec::Encode<Vec<u8>, Error = std::convert::Infallible>,
) -> Vec<u8> {
    let buf = Vec::new();
    let mut enc = Encoder::new(buf);
    match value.encode(&mut enc) {
        Ok(()) => {}
        Err(e) => match e {},
    }
    enc.into_inner()
}

/// Decode a value directly from bytes, skipping the intermediate `Value` representation.
pub fn decode_obj<T: codec::Decode>(data: &[u8]) -> core::result::Result<T, codec::DecodeError> {
    let mut de = codec::BinaryDeserializer::new(data);
    let result = T::decode(&mut de)?;
    if !de.is_empty() {
        return Err(BinDecodeError::UnexpectedEnd {
            expected: 0,
            available: de.remaining(),
        }
        .into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, str::FromStr};

    use crate as calldata;

    use super::*;

    #[derive(Decode)]
    struct Foo {
        a: calldata::Value,
    }

    #[test]
    fn test_nested_value_in_struct() {
        let vals = vec![
            Value::Null,
            Value::Address(Address::from([1; 20])),
            Value::Bool(false),
            Value::Bool(true),
            Value::Str("test".to_string()),
            Value::Bytes(vec![1, 2, 3]),
            Value::Number(num_bigint::BigInt::from(42)),
            Value::Number(num_bigint::BigInt::from(-42)),
            Value::Map(BTreeMap::new()),
            Value::Array(vec![Value::Null]),
        ];

        for val in &vals {
            let wrapped = calldata::Value::Map(BTreeMap::from([("a".to_owned(), val.clone())]));
            let foo: Foo =
                calldata::from_value(wrapped).expect("Failed to deserialize nested Value");

            assert_eq!(&foo.a, val);
        }

        for val in &vals {
            let val = calldata::Value::Array(vec![val.clone()]);

            let wrapped = calldata::Value::Map(BTreeMap::from([("a".to_owned(), val.clone())]));
            let foo: Foo =
                calldata::from_value(wrapped).expect("Failed to deserialize nested Value");

            assert_eq!(foo.a, val);
        }

        for val in &vals {
            let val = calldata::Value::Map(BTreeMap::from([("x".to_owned(), val.clone())]));

            let wrapped = calldata::Value::Map(BTreeMap::from([("a".to_owned(), val.clone())]));
            let foo: Foo =
                calldata::from_value(wrapped).expect("Failed to deserialize nested Value");

            assert_eq!(foo.a, val);
        }
    }

    #[derive(Decode)]
    struct FooArr {
        a: Vec<calldata::Value>,
    }

    #[test]
    fn test_nested_value_in_array() {
        let vals = vec![
            Value::Null,
            Value::Address(Address::from([1; 20])),
            Value::Bool(false),
            Value::Bool(true),
            Value::Str("test".to_string()),
            Value::Bytes(vec![1, 2, 3]),
            Value::Number(num_bigint::BigInt::from(42)),
            Value::Number(num_bigint::BigInt::from(-42)),
            Value::Map(BTreeMap::new()),
            Value::Array(vec![Value::Null]),
        ];

        for val in &vals {
            let wrapped = calldata::Value::Map(BTreeMap::from([(
                "a".to_owned(),
                calldata::Value::Array(vec![val.clone()]),
            )]));
            let foo: FooArr =
                calldata::from_value(wrapped).expect("Failed to deserialize nested Value");

            assert_eq!(foo.a.len(), 1);
            assert_eq!(&foo.a[0], val);
        }
    }

    #[derive(Decode)]
    struct FooMap {
        a: BTreeMap<String, calldata::Value>,
    }

    #[test]
    fn test_nested_value_in_map() {
        let vals = vec![
            Value::Null,
            Value::Address(Address::from([1; 20])),
            Value::Bool(false),
            Value::Bool(true),
            Value::Str("test".to_string()),
            Value::Bytes(vec![1, 2, 3]),
            Value::Number(num_bigint::BigInt::from(42)),
            Value::Number(num_bigint::BigInt::from(-42)),
            Value::Map(BTreeMap::new()),
            Value::Array(vec![Value::Null]),
        ];

        for val in &vals {
            let wrapped = calldata::Value::Map(BTreeMap::from([(
                "a".to_owned(),
                calldata::Value::Map(BTreeMap::from([("field".to_owned(), val.clone())])),
            )]));
            let foo: FooMap =
                calldata::from_value(wrapped).expect("Failed to deserialize nested Value");

            assert_eq!(foo.a.len(), 1);
            let item = foo.a.iter().next().unwrap();
            assert_eq!(item.1, val);
        }
    }

    #[derive(Decode)]
    struct Bar {
        a: primitive_types::U256,
    }

    #[test]
    fn test_u256_ok() {
        let create = |v| calldata::Value::Map(BTreeMap::from([("a".to_owned(), Value::Number(v))]));

        let ok_list = vec![
            num_bigint::BigInt::from(0),
            num_bigint::BigInt::from(42),
            num_bigint::BigInt::from_str(
                "57896044618658097711785492504343953926634992332820282019728792003956564819968",
            )
            .unwrap(),
            num_bigint::BigInt::from_str(
                "115792089237316195423570985008687907853269984665640564039457584007913129639935",
            )
            .unwrap(),
        ];
        for ok in ok_list {
            let bar: Bar =
                calldata::from_value(create(ok.clone())).expect("Failed to deserialize U256");

            let as_str = ok.to_str_radix(16);
            let expected = primitive_types::U256::from_str_radix(&as_str, 16).unwrap();

            assert_eq!(bar.a, expected);
        }
    }

    #[test]
    fn test_u256_not_ok() {
        let create = |v| calldata::Value::Map(BTreeMap::from([("a".to_owned(), Value::Number(v))]));

        let ok_list = vec![
            num_bigint::BigInt::from(-42),
            num_bigint::BigInt::from_str(
                "115792089237316195423570985008687907853269984665640564039457584007913129639936",
            )
            .unwrap(),
        ];
        for ok in ok_list {
            assert!(calldata::from_value::<Bar>(create(ok.clone())).is_err());

            let as_str = ok.to_str_radix(16);
            assert!(primitive_types::U256::from_str_radix(&as_str, 16).is_err());
        }
    }
}
