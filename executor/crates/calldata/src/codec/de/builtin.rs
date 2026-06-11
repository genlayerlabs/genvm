//! [`Decode`] implementations for built-in and standard-library types.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::Address;

use super::{Decode, DecodeError, Deserializer, MapAccess, SeqAccess, Visitor};

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
                while let Some(()) = seq.next_element_validate::<T>()? {}
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
            fn visit_map<A: MapAccess>(self, _len: u64, mut map: A) -> Result<(), DecodeError> {
                while let Some(()) = map.next_element_validate::<T>()? {}
                Ok(())
            }
        }
        let v: V<T> = V(std::marker::PhantomData);
        deserializer.deserialize(v)
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
