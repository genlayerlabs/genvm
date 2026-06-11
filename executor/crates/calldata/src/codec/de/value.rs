//! Decoding to/from the in-memory [`Value`] tree.
//!
//! [`ValueDeserializer`] consumes an owned `Value` (moving leaves out), while
//! [`RefValueDeserializer`] walks a borrowed `&Value` without deep-cloning
//! arrays/maps — so validating a borrowed `Value` retains only `O(depth)`.

use std::collections::BTreeMap;

use crate::{Address, Value};

use super::{Decode, DecodeError, Deserializer, MapAccess, SeqAccess, Visitor};

/// Dispatch a visitor over a borrowed [`Value`], walking containers by reference.
pub(super) fn dispatch_ref<V: Visitor>(visitor: V, value: &Value) -> Result<V::Value, DecodeError> {
    match value {
        Value::Null => visitor.visit_null(),
        Value::Bool(v) => visitor.visit_bool(*v),
        Value::Address(v) => visitor.visit_address(v),
        Value::Number(v) => visitor.visit_bigint(v),
        Value::Str(v) => visitor.visit_str(v),
        Value::Bytes(v) => visitor.visit_bytes(v),
        Value::Array(arr) => visitor.visit_seq(arr.len() as u64, RefValueSeqAccess(arr.iter())),
        Value::Map(map) => visitor.visit_map(
            map.len() as u64,
            RefValueMapAccess {
                iter: map.iter(),
                pending: None,
            },
        ),
    }
}

/// Dispatch a visitor over an owned [`Value`], moving containers into the access iterators.
pub(super) fn dispatch_owned<V: Visitor>(
    visitor: V,
    value: Value,
) -> Result<V::Value, DecodeError> {
    match value {
        Value::Null => visitor.visit_null(),
        Value::Bool(v) => visitor.visit_bool(v),
        Value::Address(v) => visitor.visit_address(&v),
        Value::Number(v) => visitor.visit_bigint(&v),
        Value::Str(v) => visitor.visit_str(&v),
        Value::Bytes(v) => visitor.visit_bytes(&v),
        Value::Array(arr) => visitor.visit_seq(arr.len() as u64, ValueSeqAccess(arr.into_iter())),
        Value::Map(map) => visitor.visit_map(
            map.len() as u64,
            ValueMapAccess {
                iter: map.into_iter(),
                current_key: String::new(),
                pending: None,
            },
        ),
    }
}

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
                while let Some(()) = seq.next_element_validate::<Value>()? {}
                Ok(())
            }

            fn visit_map<A: MapAccess>(self, _len: u64, mut map: A) -> Result<(), DecodeError> {
                while let Some(()) = map.next_element_validate::<Value>()? {}
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

// ValueDeserializer — decode/validate from an owned Value (moving leaves out)

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
    pending: Option<Value>,
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

    fn next_key(&mut self) -> Result<Option<&str>, DecodeError> {
        match self.iter.next() {
            Some((k, v)) => {
                self.current_key = k;
                self.pending = Some(v);
                Ok(Some(&self.current_key))
            }
            None => Ok(None),
        }
    }

    fn next_value<T: Decode>(&mut self) -> Result<T, DecodeError> {
        let value = self.pending.take().expect("next_value without next_key");
        T::decode(ValueDeserializer(value))
    }

    fn next_value_visit<V: Visitor>(&mut self, visitor: V) -> Result<V::Value, DecodeError> {
        let value = self
            .pending
            .take()
            .expect("next_value_visit without next_key");
        ValueDeserializer(value).deserialize(visitor)
    }
}

// RefValueDeserializer — decode/validate from a borrowed Value without cloning

/// A [`Deserializer`] over a borrowed [`Value`]. Walks the tree by reference, so
/// validating (or decoding) does not deep-clone arrays/maps; only retained leaf
/// scalars are copied (and validation copies nothing).
pub struct RefValueDeserializer<'a>(pub &'a Value);

impl Deserializer for RefValueDeserializer<'_> {
    fn deserialize<V: Visitor>(self, visitor: V) -> Result<V::Value, DecodeError> {
        visitor.visit_value(self.0)
    }
}

struct RefValueSeqAccess<'a>(std::slice::Iter<'a, Value>);

impl SeqAccess for RefValueSeqAccess<'_> {
    fn next_element<T: Decode>(&mut self) -> Result<Option<T>, DecodeError> {
        match self.0.next() {
            Some(v) => T::decode(RefValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }

    fn next_element_validate<T: Decode>(&mut self) -> Result<Option<()>, DecodeError> {
        match self.0.next() {
            Some(v) => T::validate(RefValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }
}

struct RefValueMapAccess<'a> {
    iter: std::collections::btree_map::Iter<'a, String, Value>,
    pending: Option<&'a Value>,
}

impl MapAccess for RefValueMapAccess<'_> {
    fn next_element<T: Decode>(&mut self) -> Result<Option<(&str, T)>, DecodeError> {
        match self.iter.next() {
            Some((k, v)) => {
                let val = T::decode(RefValueDeserializer(v))?;
                Ok(Some((k.as_str(), val)))
            }
            None => Ok(None),
        }
    }

    fn next_element_validate<T: Decode>(&mut self) -> Result<Option<()>, DecodeError> {
        match self.iter.next() {
            Some((_k, v)) => T::validate(RefValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }

    fn next_key(&mut self) -> Result<Option<&str>, DecodeError> {
        match self.iter.next() {
            Some((k, v)) => {
                self.pending = Some(v);
                Ok(Some(k.as_str()))
            }
            None => Ok(None),
        }
    }

    fn next_value<T: Decode>(&mut self) -> Result<T, DecodeError> {
        let value = self.pending.take().expect("next_value without next_key");
        T::decode(RefValueDeserializer(value))
    }

    fn next_value_visit<V: Visitor>(&mut self, visitor: V) -> Result<V::Value, DecodeError> {
        let value = self
            .pending
            .take()
            .expect("next_value_visit without next_key");
        RefValueDeserializer(value).deserialize(visitor)
    }
}
