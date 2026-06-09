use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{Address, BinDecodeError, Value, ValueKind};

pub enum DecodeError {
    FieldMissing(&'static str),
    FieldOutOfOrder(&'static str),
    Unexpected(&'static str),
    UnexpectedKind(ValueKind),

    // schema-level
    UnknownField(String),
    UnknownVariant { got: String, expected: &'static str },
    LengthMismatch { expected: usize, got: usize },

    // value-level
    OutOfRange { value: String, target: &'static str },

    // wire-level
    // user-provided
    UserError(Box<dyn std::error::Error + Send + Sync>),
    // fallback
    Custom(String),
    BinDecodeError(crate::bin::BinDecodeError),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::FieldMissing(field) => write!(f, "field missing: {field}"),
            DecodeError::FieldOutOfOrder(field) => write!(f, "field out of order: {field}"),
            DecodeError::Unexpected(msg) => write!(f, "unexpected: {msg}"),
            DecodeError::UnexpectedKind(kind) => write!(f, "unexpected {kind}"),
            DecodeError::UnknownField(field) => write!(f, "unknown field `{field}`"),
            DecodeError::UnknownVariant { got, expected } => {
                write!(f, "unknown variant `{got}`, expected one of: {expected}")
            }
            DecodeError::LengthMismatch { expected, got } => {
                write!(f, "expected {expected} elements, got {got}")
            }
            DecodeError::OutOfRange { value, target } => {
                write!(f, "value {value} out of range for {target}")
            }
            DecodeError::UserError(e) => write!(f, "{e}"),
            DecodeError::Custom(msg) => write!(f, "{msg}"),
            DecodeError::BinDecodeError(e) => write!(f, "Binary Error: {e}"),
        }
    }
}

impl std::fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}

impl From<crate::bin::BinDecodeError> for DecodeError {
    fn from(e: crate::bin::BinDecodeError) -> Self {
        DecodeError::BinDecodeError(e)
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::BinDecodeError(e) => Some(e),
            DecodeError::UserError(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

pub trait Decode: Sized {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError>;

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        Self::decode(deserializer).map(|_| ())
    }
}

pub trait Deserializer: Sized {
    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError>;
}

pub trait SeqAccess {
    fn next_element<T>(&mut self) -> Result<Option<T>, DecodeError>
    where
        T: Decode;

    fn next_element_validate<T>(&mut self) -> Result<Option<()>, DecodeError>
    where
        T: Decode,;
}

pub trait MapAccess {
    fn next_element<T>(&mut self) -> Result<Option<(&str, T)>, DecodeError>
    where
        T: Decode;

    fn next_element_validate<T>(&mut self) -> Result<Option<()>, DecodeError>
    where
        T: Decode,;
}

pub trait Visitor: Sized {
    type Value;

    fn visit_bool(self, _value: bool) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Bool))
    }

    fn visit_null(self) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Null))
    }

    fn visit_address(self, _value: &Address) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Address))
    }

    fn visit_bigint(self, _value: &num_bigint::BigInt) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Number))
    }

    fn visit_bigint_owned(self, value: num_bigint::BigInt) -> Result<Self::Value, DecodeError> {
        self.visit_bigint(&value)
    }

    fn visit_i64(self, value: i64) -> Result<Self::Value, DecodeError> {
        self.visit_bigint_owned(num_bigint::BigInt::from(value))
    }

    fn visit_u64(self, value: u64) -> Result<Self::Value, DecodeError> {
        self.visit_bigint_owned(num_bigint::BigInt::from(value))
    }

    fn visit_str(self, _value: &str) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Str))
    }

    fn visit_bytes(self, _value: &[u8]) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Bytes))
    }

    fn visit_seq<A: SeqAccess>(self, _len: u64, _seq: A) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Array))
    }

    fn visit_map<A: MapAccess>(self, _len: u64, _map: A) -> Result<Self::Value, DecodeError> {
        Err(DecodeError::UnexpectedKind(ValueKind::Map))
    }

    fn visit_value(self, value: &Value) -> Result<Self::Value, DecodeError> {
        match value {
            Value::Null => self.visit_null(),
            Value::Bool(v) => self.visit_bool(*v),
            Value::Address(v) => self.visit_address(v),
            Value::Number(v) => self.visit_bigint(v),
            Value::Str(v) => self.visit_str(v),
            Value::Bytes(v) => self.visit_bytes(v),
            Value::Array(arr) => {
                let len = arr.len() as u64;
                self.visit_seq(len, ValueSeqAccess(arr.clone().into_iter()))
            }
            Value::Map(map) => {
                let len = map.len() as u64;
                self.visit_map(
                    len,
                    ValueMapAccess {
                        iter: map.clone().into_iter(),
                        current_key: String::new(),
                    },
                )
            }
        }
    }

    fn visit_value_owned(self, value: Value) -> Result<Self::Value, DecodeError> {
        match value {
            Value::Null => self.visit_null(),
            Value::Bool(v) => self.visit_bool(v),
            Value::Address(v) => self.visit_address(&v),
            Value::Number(v) => self.visit_bigint(&v),
            Value::Str(v) => self.visit_str(&v),
            Value::Bytes(v) => self.visit_bytes(&v),
            Value::Array(arr) => {
                let len = arr.len() as u64;
                self.visit_seq(len, ValueSeqAccess(arr.into_iter()))
            }
            Value::Map(map) => {
                let len = map.len() as u64;
                self.visit_map(
                    len,
                    ValueMapAccess {
                        iter: map.into_iter(),
                        current_key: String::new(),
                    },
                )
            }
        }
    }
}

// bool

impl Decode for bool {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = bool;
            fn visit_bool(self, value: bool) -> Result<bool, DecodeError> {
                Ok(value)
            }
        }
        deserializer.deserialize(V)
    }
}

// integers via macro

macro_rules! impl_decode_int {
    ($($ty:ty),*) => {
        $(
            impl Decode for $ty {
                fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
                    struct V;
                    impl Visitor for V {
                        type Value = $ty;
                        fn visit_i64(self, value: i64) -> Result<$ty, DecodeError> {
                            <$ty>::try_from(value).map_err(|_| DecodeError::OutOfRange {
                                value: value.to_string(),
                                target: stringify!($ty),
                            })
                        }
                        fn visit_u64(self, value: u64) -> Result<$ty, DecodeError> {
                            <$ty>::try_from(value).map_err(|_| DecodeError::OutOfRange {
                                value: value.to_string(),
                                target: stringify!($ty),
                            })
                        }
                        fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<$ty, DecodeError> {
                            <$ty>::try_from(value).map_err(|_| DecodeError::OutOfRange {
                                value: value.to_string(),
                                target: stringify!($ty),
                            })
                        }
                    }
                    deserializer.deserialize(V)
                }
            }
        )*
    };
}

impl_decode_int!(i8, i16, i32, i64, u8, u16, u32, u64);

// String

impl Decode for String {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = String;
            fn visit_str(self, value: &str) -> Result<String, DecodeError> {
                Ok(value.to_owned())
            }
        }
        deserializer.deserialize(V)
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = ();
            fn visit_str(self, _value: &str) -> Result<(), DecodeError> {
                Ok(())
            }
        }
        deserializer.deserialize(V)
    }
}

// bytes::Bytes

impl Decode for bytes::Bytes {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = bytes::Bytes;
            fn visit_bytes(self, value: &[u8]) -> Result<bytes::Bytes, DecodeError> {
                Ok(bytes::Bytes::copy_from_slice(value))
            }
        }
        deserializer.deserialize(V)
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = ();
            fn visit_bytes(self, _value: &[u8]) -> Result<(), DecodeError> {
                Ok(())
            }
        }
        deserializer.deserialize(V)
    }
}

// BigInt

impl Decode for num_bigint::BigInt {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = num_bigint::BigInt;
            fn visit_bigint(
                self,
                value: &num_bigint::BigInt,
            ) -> Result<num_bigint::BigInt, DecodeError> {
                Ok(value.clone())
            }
            fn visit_bigint_owned(
                self,
                value: num_bigint::BigInt,
            ) -> Result<num_bigint::BigInt, DecodeError> {
                Ok(value)
            }
        }
        deserializer.deserialize(V)
    }
}

// U256

impl Decode for primitive_types::U256 {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = primitive_types::U256;

            fn visit_bigint(
                self,
                value: &num_bigint::BigInt,
            ) -> Result<primitive_types::U256, DecodeError> {
                use num_bigint::Sign;
                let (sign, bytes) = value.to_bytes_le();
                if sign == Sign::Minus {
                    return Err(DecodeError::OutOfRange {
                        value: value.to_string(),
                        target: "U256",
                    });
                }
                if bytes.len() > 32 {
                    return Err(DecodeError::OutOfRange {
                        value: value.to_string(),
                        target: "U256",
                    });
                }
                Ok(primitive_types::U256::from_little_endian(&bytes))
            }
        }
        deserializer.deserialize(V)
    }
}

// Address

impl Decode for Address {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = Address;
            fn visit_address(self, value: &Address) -> Result<Address, DecodeError> {
                Ok(*value)
            }
        }
        deserializer.deserialize(V)
    }
}

// Option<T>

impl<T: Decode> Decode for Option<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = Option<T>;

            fn visit_null(self) -> Result<Option<T>, DecodeError> {
                Ok(None)
            }

            fn visit_bool(self, value: bool) -> Result<Option<T>, DecodeError> {
                struct Wrap(bool);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_bool(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_address(self, value: &Address) -> Result<Option<T>, DecodeError> {
                struct Wrap(Address);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_address(&self.0)
                    }
                }
                T::decode(Wrap(*value)).map(Some)
            }

            fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<Option<T>, DecodeError> {
                struct Wrap(num_bigint::BigInt);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_bigint_owned(self.0)
                    }
                }
                T::decode(Wrap(value.clone())).map(Some)
            }

            fn visit_bigint_owned(
                self,
                value: num_bigint::BigInt,
            ) -> Result<Option<T>, DecodeError> {
                struct Wrap(num_bigint::BigInt);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_bigint_owned(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_i64(self, value: i64) -> Result<Option<T>, DecodeError> {
                struct Wrap(i64);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_i64(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_u64(self, value: u64) -> Result<Option<T>, DecodeError> {
                struct Wrap(u64);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_u64(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_str(self, value: &str) -> Result<Option<T>, DecodeError> {
                struct Wrap(String);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_str(&self.0)
                    }
                }
                T::decode(Wrap(value.to_owned())).map(Some)
            }

            fn visit_bytes(self, value: &[u8]) -> Result<Option<T>, DecodeError> {
                struct Wrap(Vec<u8>);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_bytes(&self.0)
                    }
                }
                T::decode(Wrap(value.to_vec())).map(Some)
            }

            fn visit_seq<A: SeqAccess>(self, len: u64, seq: A) -> Result<Option<T>, DecodeError> {
                struct Wrap<A: SeqAccess> {
                    len: u64,
                    seq: A,
                }
                impl<A: SeqAccess> Deserializer for Wrap<A> {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_seq(self.len, self.seq)
                    }
                }
                T::decode(Wrap { len, seq }).map(Some)
            }

            fn visit_map<A: MapAccess>(self, len: u64, map: A) -> Result<Option<T>, DecodeError> {
                struct Wrap<A: MapAccess> {
                    len: u64,
                    map: A,
                }
                impl<A: MapAccess> Deserializer for Wrap<A> {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
                        visitor.visit_map(self.len, self.map)
                    }
                }
                T::decode(Wrap { len, map }).map(Some)
            }
        }
        deserializer.deserialize(V(std::marker::PhantomData))
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = Vec<T>;
            fn visit_seq<A: SeqAccess>(self, len: u64, mut seq: A) -> Result<Vec<T>, DecodeError> {
                let mut result = Vec::with_capacity(len as usize);
                while let Some(elem) = seq.next_element::<T>()? {
                    result.push(elem);
                }
                Ok(result)
            }
        }
        deserializer.deserialize(V(std::marker::PhantomData))
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = ();
            fn visit_seq<A: SeqAccess>(self, _len: u64, mut seq: A) -> Result<(), DecodeError> {
                while let Some(()) = seq.next_element_validate::<T>()? {
                }
                Ok(())
            }
        }
        let v: V<T> = V(std::marker::PhantomData);
        deserializer.deserialize(v)
    }
}

// BTreeMap<String, T>

impl<T: Decode> Decode for BTreeMap<String, T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = BTreeMap<String, T>;
            fn visit_map<A: MapAccess>(
                self,
                _len: u64,
                mut map: A,
            ) -> Result<BTreeMap<String, T>, DecodeError> {
                let mut result = BTreeMap::new();
                while let Some((key, value)) = map.next_element::<T>()? {
                    result.insert(key.to_owned(), value);
                }
                Ok(result)
            }
        }
        deserializer.deserialize(V(std::marker::PhantomData))
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = ();
            fn visit_map<A: MapAccess>(
                self,
                _len: u64,
                mut map: A,
            ) -> Result<(), DecodeError> {
                while let Some(()) = map.next_element_validate::<T>()? {
                }
                Ok(())
            }
        }
        let v: V<T> = V(std::marker::PhantomData);
        deserializer.deserialize(v)
    }
}

// Value

impl Decode for Value {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = Value;

            fn visit_bool(self, value: bool) -> Result<Value, DecodeError> {
                Ok(Value::Bool(value))
            }

            fn visit_null(self) -> Result<Value, DecodeError> {
                Ok(Value::Null)
            }

            fn visit_address(self, value: &Address) -> Result<Value, DecodeError> {
                Ok(Value::Address(*value))
            }

            fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<Value, DecodeError> {
                Ok(Value::Number(value.clone()))
            }

            fn visit_bigint_owned(self, value: num_bigint::BigInt) -> Result<Value, DecodeError> {
                Ok(Value::Number(value))
            }

            fn visit_str(self, value: &str) -> Result<Value, DecodeError> {
                Ok(Value::Str(value.to_owned()))
            }

            fn visit_bytes(self, value: &[u8]) -> Result<Value, DecodeError> {
                Ok(Value::Bytes(value.to_vec()))
            }

            fn visit_seq<A: SeqAccess>(self, len: u64, mut seq: A) -> Result<Value, DecodeError> {
                let mut result = Vec::with_capacity(len as usize);
                while let Some(elem) = seq.next_element::<Value>()? {
                    result.push(elem);
                }
                Ok(Value::Array(result))
            }

            fn visit_map<A: MapAccess>(self, _len: u64, mut map: A) -> Result<Value, DecodeError> {
                let mut result = BTreeMap::new();
                while let Some((key, value)) = map.next_element::<Value>()? {
                    result.insert(key.to_owned(), value);
                }
                Ok(Value::Map(result))
            }

            fn visit_value(self, value: &Value) -> Result<Value, DecodeError> {
                Ok(value.clone())
            }

            fn visit_value_owned(self, value: Value) -> Result<Value, DecodeError> {
                Ok(value)
            }
        }
        deserializer.deserialize(V)
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        struct V;
        impl Visitor for V {
            type Value = ();

            fn visit_bool(self, _value: bool) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_null(self) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_address(self, _value: &Address) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_bigint(self, _value: &num_bigint::BigInt) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_bigint_owned(self, _value: num_bigint::BigInt) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_str(self, _value: &str) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_bytes(self, _value: &[u8]) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_seq<A: SeqAccess>(self, _len: u64, mut seq: A) -> Result<(), DecodeError> {
                while let Some(()) = seq.next_element_validate::<Value>()? {
                }
                Ok(())
            }

            fn visit_map<A: MapAccess>(self, _len: u64, mut map: A) -> Result<(), DecodeError> {
                while let Some(()) = map.next_element_validate::<Value>()? {
                }
                Ok(())
            }

            fn visit_value(self, _value: &Value) -> Result<(), DecodeError> {
                Ok(())
            }

            fn visit_value_owned(self, _value: Value) -> Result<(), DecodeError> {
                Ok(())
            }
        }
        deserializer.deserialize(V)
    }
}

// Transparent wrappers

impl<T: Decode> Decode for Box<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        T::decode(deserializer).map(Box::new)
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        T::validate(deserializer)
    }
}

impl<T: Decode> Decode for Arc<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        T::decode(deserializer).map(Arc::new)
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        T::validate(deserializer)
    }
}

impl<T: Decode> Decode for std::rc::Rc<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        T::decode(deserializer).map(std::rc::Rc::new)
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        T::validate(deserializer)
    }
}

// ValueDeserializer — allows decoding concrete types from a Value

pub struct ValueDeserializer(pub Value);

impl Deserializer for ValueDeserializer {
    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
        visitor.visit_value_owned(self.0)
    }
}

struct ValueSeqAccess(std::vec::IntoIter<Value>);

impl SeqAccess for ValueSeqAccess {
    fn next_element<T: Decode>(&mut self) -> Result<Option<T>, DecodeError> {
        match self.0.next() {
            Some(v) => T::decode(ValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }

    fn next_element_validate<T: Decode>(&mut self) -> Result<Option<()>, DecodeError> {
        match self.0.next() {
            Some(v) => T::validate(ValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }
}

struct ValueMapAccess {
    iter: std::collections::btree_map::IntoIter<String, Value>,
    current_key: String,
}

impl MapAccess for ValueMapAccess {
    fn next_element<T: Decode>(&mut self) -> Result<Option<(&str, T)>, DecodeError> {
        match self.iter.next() {
            Some((k, v)) => {
                self.current_key = k;
                let val = T::decode(ValueDeserializer(v))?;
                Ok(Some((&self.current_key, val)))
            }
            None => Ok(None),
        }
    }

    fn next_element_validate<T: Decode>(&mut self) -> Result<Option<()>, DecodeError> {
        match self.iter.next() {
            Some((k, v)) => {
                self.current_key = k;
                T::validate(ValueDeserializer(v)).map(Some)
            }
            None => Ok(None),
        }
    }
}

// BinaryDeserializer — reads the calldata wire format directly

use crate::consts::*;

/// Options for [`BinaryDeserializer`].
#[derive(Debug, Clone)]
pub struct BinaryDeserializerOptions {
    pub max_depth: usize,
}

impl Default for BinaryDeserializerOptions {
    fn default() -> Self {
        Self { max_depth: 128 }
    }
}

pub struct BinaryDeserializer<'a> {
    data: &'a [u8],
    depth: usize,
    opts: BinaryDeserializerOptions,
}

impl<'a> BinaryDeserializer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            depth: 0,
            opts: BinaryDeserializerOptions::default(),
        }
    }

    pub fn with_options(data: &'a [u8], opts: BinaryDeserializerOptions) -> Self {
        Self {
            data,
            depth: 0,
            opts,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.data.len()
    }

    fn fetch_byte(&mut self) -> Result<u8, DecodeError> {
        if self.data.is_empty() {
            return Err(BinDecodeError::UnexpectedEnd {
                expected: 1,
                available: 0,
            }
            .into());
        }
        let b = self.data[0];
        self.data = &self.data[1..];
        Ok(b)
    }

    fn fetch_slice(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.data.len() < n {
            return Err(BinDecodeError::UnexpectedEnd {
                expected: n,
                available: self.data.len(),
            }
            .into());
        }
        let (head, tail) = self.data.split_at(n);
        self.data = tail;
        Ok(head)
    }

    fn fetch_uleb(&mut self) -> Result<num_bigint::BigUint, DecodeError> {
        let mut res = num_bigint::BigUint::ZERO;
        let mut off = 0u64;
        loop {
            let byte = self.fetch_byte()?;
            res += num_bigint::BigUint::from(byte & 0x7f) << off;
            if byte & 0x80 == 0 {
                if byte == 0 && off != 0 {
                    return Err(BinDecodeError::InvalidUlebEncoding.into());
                }
                return Ok(res);
            }
            off = off.checked_add(7).ok_or(BinDecodeError::NumberTooBig)?;
        }
    }

    fn uleb_to_usize(val: &num_bigint::BigUint) -> Result<usize, DecodeError> {
        if val.bits() > 32 {
            return Err(BinDecodeError::ContainerSizeTooLarge { bits: val.bits() }.into());
        }
        Ok(val.to_u32_digits().first().cloned().unwrap_or(0) as usize)
    }

    fn fetch_map_key(&mut self) -> Result<&'a str, DecodeError> {
        let key_len = self.fetch_uleb()?;
        let key_len = Self::uleb_to_usize(&key_len)?;
        let key_bytes = self.fetch_slice(key_len)?;
        std::str::from_utf8(key_bytes)
            .map_err(BinDecodeError::InvalidUtf8)
            .map_err(Into::into)
    }

    fn deserialize_one<V: Visitor>(&mut self, visitor: V) -> Result<V::Value, DecodeError> {
        let mut val = self.fetch_uleb()?;
        let least = (val.iter_u32_digits().next().unwrap_or(0) & (u8::MAX as u32)) as u8;
        let typ = least & ((1 << BITS_IN_TYPE) - 1);
        val >>= BITS_IN_TYPE;

        match typ {
            TYPE_SPECIAL => {
                if val.bits() > 8 - BITS_IN_TYPE as u64 {
                    return Err(BinDecodeError::InvalidTag(least).into());
                }
                match least {
                    SPECIAL_NULL => visitor.visit_null(),
                    SPECIAL_TRUE => visitor.visit_bool(true),
                    SPECIAL_FALSE => visitor.visit_bool(false),
                    SPECIAL_ADDR => {
                        let slice = self.fetch_slice(Address::SIZE as usize)?;
                        let mut raw = [0u8; Address::SIZE as usize];
                        raw.copy_from_slice(slice);
                        visitor.visit_address(&Address::from(raw))
                    }
                    other => Err(BinDecodeError::InvalidTag(other).into()),
                }
            }
            TYPE_PINT => {
                let n = num_bigint::BigInt::from_biguint(num_bigint::Sign::Plus, val);
                visitor.visit_bigint_owned(n)
            }
            TYPE_NINT => {
                val += 1u32;
                let n = num_bigint::BigInt::from_biguint(num_bigint::Sign::Minus, val);
                visitor.visit_bigint_owned(n)
            }
            TYPE_BYTES => {
                let len = Self::uleb_to_usize(&val)?;
                let slice = self.fetch_slice(len)?;
                visitor.visit_bytes(slice)
            }
            TYPE_STR => {
                let len = Self::uleb_to_usize(&val)?;
                let slice = self.fetch_slice(len)?;
                let s = std::str::from_utf8(slice).map_err(BinDecodeError::InvalidUtf8)?;
                visitor.visit_str(s)
            }
            TYPE_ARR => {
                if self.depth >= self.opts.max_depth {
                    return Err(BinDecodeError::MaxDepthExceeded.into());
                }
                let len = Self::uleb_to_usize(&val)?;
                if self.remaining() < len {
                    return Err(BinDecodeError::UnexpectedEnd {
                        expected: len,
                        available: self.remaining(),
                    }
                    .into());
                }
                self.depth += 1;
                let result = visitor.visit_seq(
                    len as u64,
                    BinarySeqAccess {
                        de: self,
                        remaining: len as u64,
                    },
                );
                self.depth -= 1;
                result
            }
            TYPE_MAP => {
                if self.depth >= self.opts.max_depth {
                    return Err(BinDecodeError::MaxDepthExceeded.into());
                }
                let len = Self::uleb_to_usize(&val)?;
                // Each map entry needs at least 2 bytes (key len + value tag).
                if self.remaining() < len.saturating_mul(2) {
                    return Err(BinDecodeError::UnexpectedEnd {
                        expected: len.saturating_mul(2),
                        available: self.remaining(),
                    }
                    .into());
                }
                self.depth += 1;
                let result = visitor.visit_map(
                    len as u64,
                    BinaryMapAccess {
                        de: self,
                        remaining: len as u64,
                        prev_key: None,
                    },
                );
                self.depth -= 1;
                result
            }
            other => Err(BinDecodeError::InvalidTag(other).into()),
        }
    }
}

impl Deserializer for &mut BinaryDeserializer<'_> {
    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
        self.deserialize_one(visitor)
    }
}

impl Deserializer for BinaryDeserializer<'_> {
    fn deserialize<V: Visitor>(mut self, visitor: V) -> Result<V::Value, DecodeError> {
        let result = self.deserialize_one(visitor)?;
        if !self.data.is_empty() {
            return Err(BinDecodeError::UnexpectedEnd {
                available: self.data.len(),
                expected: 0,
            }
            .into());
        }
        Ok(result)
    }
}

struct BinarySeqAccess<'a, 'b> {
    de: &'b mut BinaryDeserializer<'a>,
    remaining: u64,
}

impl SeqAccess for BinarySeqAccess<'_, '_> {
    fn next_element<T: Decode>(&mut self) -> Result<Option<T>, DecodeError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        T::decode(&mut *self.de).map(Some)
    }

    fn next_element_validate<T>(&mut self) -> Result<Option<()>, DecodeError>
    where
        T: Decode,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        T::validate(&mut *self.de).map(Some)
    }
}

struct BinaryMapAccess<'a, 'b> {
    de: &'b mut BinaryDeserializer<'a>,
    remaining: u64,
    prev_key: Option<&'a str>,
}

impl MapAccess for BinaryMapAccess<'_, '_> {
    fn next_element<T: Decode>(&mut self) -> Result<Option<(&str, T)>, DecodeError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        let key = self.de.fetch_map_key()?;
        if let Some(prev) = self.prev_key
            && prev >= key
        {
            return Err(BinDecodeError::InvalidMapOrdering {
                prev: prev.to_owned(),
                current: key.to_owned(),
            }
            .into());
        }
        self.prev_key = Some(key);
        let val = T::decode(&mut *self.de)?;
        Ok(Some((key, val)))
    }

    fn next_element_validate<T: Decode>(&mut self) -> Result<Option<()>, DecodeError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        let key = self.de.fetch_map_key()?;
        if let Some(prev) = self.prev_key
            && prev >= key
        {
            return Err(BinDecodeError::InvalidMapOrdering {
                prev: prev.to_owned(),
                current: key.to_owned(),
            }
            .into());
        }
        self.prev_key = Some(key);
        T::validate(&mut *self.de).map(Some)
    }
}
