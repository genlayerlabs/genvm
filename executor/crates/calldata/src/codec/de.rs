use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{Address, Value};

pub enum Error {
    FieldMissing(&'static str),
    FieldOutOfOrder(&'static str),
    UnparsedEnd,
    Unexpected(&'static str),
    Custom(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FieldMissing(field) => write!(f, "Field missing: {field}"),
            Error::FieldOutOfOrder(field) => write!(f, "Field out of order: {field}"),
            Error::UnparsedEnd => write!(f, "Unparsed end"),
            Error::Unexpected(msg) => write!(f, "Unexpected: {msg}"),
            Error::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}

impl std::error::Error for Error {}

pub trait Decode: Sized {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error>;
}

pub trait Deserializer: Sized {
    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error>;
}

pub trait SeqAccess {
    fn next_element<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: Decode;
}

pub trait MapAccess {
    fn next_element<T>(&mut self) -> Result<Option<(&str, T)>, Error>
    where
        T: Decode;
}

pub trait Visitor: Sized {
    type Value;

    fn visit_bool(self, _value: bool) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("bool"))
    }

    fn visit_null(self) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("null"))
    }

    fn visit_address(self, _value: &Address) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("address"))
    }

    fn visit_bigint(self, _value: &num_bigint::BigInt) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("bigint"))
    }

    fn visit_bigint_owned(self, value: num_bigint::BigInt) -> Result<Self::Value, Error> {
        self.visit_bigint(&value)
    }

    fn visit_i64(self, value: i64) -> Result<Self::Value, Error> {
        self.visit_bigint_owned(num_bigint::BigInt::from(value))
    }

    fn visit_u64(self, value: u64) -> Result<Self::Value, Error> {
        self.visit_bigint_owned(num_bigint::BigInt::from(value))
    }

    fn visit_str(self, _value: &str) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("str"))
    }

    fn visit_bytes(self, _value: &[u8]) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("bytes"))
    }

    fn visit_seq<A: SeqAccess>(self, _len: u64, _seq: A) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("seq"))
    }

    fn visit_map<A: MapAccess>(self, _len: u64, _map: A) -> Result<Self::Value, Error> {
        Err(Error::Unexpected("map"))
    }
}

// bool

impl Decode for bool {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = bool;
            fn visit_bool(self, value: bool) -> Result<bool, Error> {
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
                fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
                    struct V;
                    impl Visitor for V {
                        type Value = $ty;
                        fn visit_i64(self, value: i64) -> Result<$ty, Error> {
                            <$ty>::try_from(value).map_err(|_| Error::Custom(
                                format!("i64 value {} out of range for {}", value, stringify!($ty))
                            ))
                        }
                        fn visit_u64(self, value: u64) -> Result<$ty, Error> {
                            <$ty>::try_from(value).map_err(|_| Error::Custom(
                                format!("u64 value {} out of range for {}", value, stringify!($ty))
                            ))
                        }
                        fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<$ty, Error> {
                            <$ty>::try_from(value).map_err(|_| Error::Custom(
                                format!("bigint value {} out of range for {}", value, stringify!($ty))
                            ))
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
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = String;
            fn visit_str(self, value: &str) -> Result<String, Error> {
                Ok(value.to_owned())
            }
        }
        deserializer.deserialize(V)
    }
}

// bytes::Bytes

impl Decode for bytes::Bytes {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = bytes::Bytes;
            fn visit_bytes(self, value: &[u8]) -> Result<bytes::Bytes, Error> {
                Ok(bytes::Bytes::copy_from_slice(value))
            }
        }
        deserializer.deserialize(V)
    }
}

// BigInt

impl Decode for num_bigint::BigInt {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = num_bigint::BigInt;
            fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<num_bigint::BigInt, Error> {
                Ok(value.clone())
            }
            fn visit_bigint_owned(
                self,
                value: num_bigint::BigInt,
            ) -> Result<num_bigint::BigInt, Error> {
                Ok(value)
            }
        }
        deserializer.deserialize(V)
    }
}

// U256

impl Decode for primitive_types::U256 {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = primitive_types::U256;

            fn visit_bigint(
                self,
                value: &num_bigint::BigInt,
            ) -> Result<primitive_types::U256, Error> {
                use num_bigint::Sign;
                let (sign, bytes) = value.to_bytes_le();
                if sign == Sign::Minus {
                    return Err(Error::Custom("negative value for U256".into()));
                }
                if bytes.len() > 32 {
                    return Err(Error::Custom("bigint too large for U256".into()));
                }
                Ok(primitive_types::U256::from_little_endian(&bytes))
            }
        }
        deserializer.deserialize(V)
    }
}

// Address

impl Decode for Address {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = Address;
            fn visit_address(self, value: &Address) -> Result<Address, Error> {
                Ok(*value)
            }
        }
        deserializer.deserialize(V)
    }
}

// Option<T>

impl<T: Decode> Decode for Option<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = Option<T>;

            fn visit_null(self) -> Result<Option<T>, Error> {
                Ok(None)
            }

            fn visit_bool(self, value: bool) -> Result<Option<T>, Error> {
                struct Wrap(bool);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_bool(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_address(self, value: &Address) -> Result<Option<T>, Error> {
                struct Wrap(Address);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_address(&self.0)
                    }
                }
                T::decode(Wrap(*value)).map(Some)
            }

            fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<Option<T>, Error> {
                struct Wrap(num_bigint::BigInt);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_bigint_owned(self.0)
                    }
                }
                T::decode(Wrap(value.clone())).map(Some)
            }

            fn visit_bigint_owned(self, value: num_bigint::BigInt) -> Result<Option<T>, Error> {
                struct Wrap(num_bigint::BigInt);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_bigint_owned(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_i64(self, value: i64) -> Result<Option<T>, Error> {
                struct Wrap(i64);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_i64(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_u64(self, value: u64) -> Result<Option<T>, Error> {
                struct Wrap(u64);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_u64(self.0)
                    }
                }
                T::decode(Wrap(value)).map(Some)
            }

            fn visit_str(self, value: &str) -> Result<Option<T>, Error> {
                struct Wrap(String);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_str(&self.0)
                    }
                }
                T::decode(Wrap(value.to_owned())).map(Some)
            }

            fn visit_bytes(self, value: &[u8]) -> Result<Option<T>, Error> {
                struct Wrap(Vec<u8>);
                impl Deserializer for Wrap {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_bytes(&self.0)
                    }
                }
                T::decode(Wrap(value.to_vec())).map(Some)
            }

            fn visit_seq<A: SeqAccess>(self, len: u64, seq: A) -> Result<Option<T>, Error> {
                struct Wrap<A: SeqAccess> {
                    len: u64,
                    seq: A,
                }
                impl<A: SeqAccess> Deserializer for Wrap<A> {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
                        visitor.visit_seq(self.len, self.seq)
                    }
                }
                T::decode(Wrap { len, seq }).map(Some)
            }

            fn visit_map<A: MapAccess>(self, len: u64, map: A) -> Result<Option<T>, Error> {
                struct Wrap<A: MapAccess> {
                    len: u64,
                    map: A,
                }
                impl<A: MapAccess> Deserializer for Wrap<A> {
                    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
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
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = Vec<T>;
            fn visit_seq<A: SeqAccess>(self, len: u64, mut seq: A) -> Result<Vec<T>, Error> {
                let mut result = Vec::with_capacity(len as usize);
                while let Some(elem) = seq.next_element::<T>()? {
                    result.push(elem);
                }
                Ok(result)
            }
        }
        deserializer.deserialize(V(std::marker::PhantomData))
    }
}

// BTreeMap<String, T>

impl<T: Decode> Decode for BTreeMap<String, T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = BTreeMap<String, T>;
            fn visit_map<A: MapAccess>(
                self,
                _len: u64,
                mut map: A,
            ) -> Result<BTreeMap<String, T>, Error> {
                let mut result = BTreeMap::new();
                while let Some((key, value)) = map.next_element::<T>()? {
                    result.insert(key.to_owned(), value);
                }
                Ok(result)
            }
        }
        deserializer.deserialize(V(std::marker::PhantomData))
    }
}

// Value

impl Decode for Value {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        struct V;
        impl Visitor for V {
            type Value = Value;

            fn visit_bool(self, value: bool) -> Result<Value, Error> {
                Ok(Value::Bool(value))
            }

            fn visit_null(self) -> Result<Value, Error> {
                Ok(Value::Null)
            }

            fn visit_address(self, value: &Address) -> Result<Value, Error> {
                Ok(Value::Address(*value))
            }

            fn visit_bigint(self, value: &num_bigint::BigInt) -> Result<Value, Error> {
                Ok(Value::Number(value.clone()))
            }

            fn visit_bigint_owned(self, value: num_bigint::BigInt) -> Result<Value, Error> {
                Ok(Value::Number(value))
            }

            fn visit_str(self, value: &str) -> Result<Value, Error> {
                Ok(Value::Str(value.to_owned()))
            }

            fn visit_bytes(self, value: &[u8]) -> Result<Value, Error> {
                Ok(Value::Bytes(value.to_vec()))
            }

            fn visit_seq<A: SeqAccess>(self, len: u64, mut seq: A) -> Result<Value, Error> {
                let mut result = Vec::with_capacity(len as usize);
                while let Some(elem) = seq.next_element::<Value>()? {
                    result.push(elem);
                }
                Ok(Value::Array(result))
            }

            fn visit_map<A: MapAccess>(self, _len: u64, mut map: A) -> Result<Value, Error> {
                let mut result = BTreeMap::new();
                while let Some((key, value)) = map.next_element::<Value>()? {
                    result.insert(key.to_owned(), value);
                }
                Ok(Value::Map(result))
            }
        }
        deserializer.deserialize(V)
    }
}

// Transparent wrappers

impl<T: Decode> Decode for Box<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        T::decode(deserializer).map(Box::new)
    }
}

impl<T: Decode> Decode for Arc<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        T::decode(deserializer).map(Arc::new)
    }
}

impl<T: Decode> Decode for std::rc::Rc<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, Error> {
        T::decode(deserializer).map(std::rc::Rc::new)
    }
}

// ValueDeserializer — allows decoding concrete types from a Value

pub struct ValueDeserializer(pub Value);

impl Deserializer for ValueDeserializer {
    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_null(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Address(v) => visitor.visit_address(&v),
            Value::Number(v) => visitor.visit_bigint_owned(v),
            Value::Str(v) => visitor.visit_str(&v),
            Value::Bytes(v) => visitor.visit_bytes(&v),
            Value::Array(v) => {
                let len = v.len() as u64;
                visitor.visit_seq(len, ValueSeqAccess(v.into_iter()))
            }
            Value::Map(v) => {
                let len = v.len() as u64;
                visitor.visit_map(
                    len,
                    ValueMapAccess {
                        iter: v.into_iter(),
                        current_key: String::new(),
                    },
                )
            }
        }
    }
}

struct ValueSeqAccess(std::vec::IntoIter<Value>);

impl SeqAccess for ValueSeqAccess {
    fn next_element<T: Decode>(&mut self) -> Result<Option<T>, Error> {
        match self.0.next() {
            Some(v) => T::decode(ValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }
}

struct ValueMapAccess {
    iter: std::collections::btree_map::IntoIter<String, Value>,
    current_key: String,
}

impl MapAccess for ValueMapAccess {
    fn next_element<T: Decode>(&mut self) -> Result<Option<(&str, T)>, Error> {
        match self.iter.next() {
            Some((k, v)) => {
                self.current_key = k;
                let val = T::decode(ValueDeserializer(v))?;
                Ok(Some((&self.current_key, val)))
            }
            None => Ok(None),
        }
    }
}
