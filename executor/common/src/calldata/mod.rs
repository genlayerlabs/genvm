mod bin;
mod de;
mod error;
mod se;
mod types;

pub use bin::{decode, encode};
pub use error::*;
pub use types::*;

pub fn from_value<T>(value: Value) -> core::result::Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}

pub fn to_value<T>(value: &T) -> Result<Value, Error>
where
    T: ?Sized + serde::ser::Serialize,
{
    let full_type_name = std::any::type_name::<T>();
    match full_type_name {
        "num_bigint::bigint::BigInt" => {
            let as_ptr = std::ptr::from_ref(value) as *mut num_bigint::BigInt; // should be const but non-null...
            Ok(Value::Number(
                unsafe { std::ptr::NonNull::new_unchecked(as_ptr).as_ref() }.clone(),
            ))
        }
        "genvm_common::calldata::types::Value" => {
            let as_ptr = std::ptr::from_ref(value) as *mut Value; // should be const but non-null...
            Ok(unsafe { std::ptr::NonNull::new_unchecked(as_ptr).as_ref() }.clone())
        }
        "genvm_common::calldata::types::Address" => {
            let as_ptr = std::ptr::from_ref(value) as *const Address;
            Ok(Value::Address(unsafe { as_ptr.read() }))
        }
        _ => value.serialize(se::Serializer),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, str::FromStr};

    use crate::calldata;

    use super::*;

    #[derive(serde::Deserialize)]
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

    #[derive(serde::Deserialize)]
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

    #[derive(serde::Deserialize)]
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

    #[derive(serde::Deserialize)]
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
