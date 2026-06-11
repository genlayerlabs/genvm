use crate::{Address, Value, ValueKind};

mod bin;
mod builtin;
mod unparsed;
mod value;

pub use bin::*;
pub use unparsed::*;
pub use value::*;

pub enum DecodeError {
    FieldMissing(&'static str),
    FieldOutOfOrder(&'static str),
    DuplicateField(&'static str),
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
            DecodeError::DuplicateField(field) => write!(f, "duplicate field: {field}"),
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

    /// Decode the next value as a [`Maybe<T>`]. Byte-backed deserializers defer:
    /// they validate as `T` (cheap, `O(depth)` retained) and keep the raw wire
    /// bytes ([`Maybe::Checked`]). For other sources there are no bytes to keep —
    /// the in-memory value already exists — so decoding eagerly into the (usually
    /// smaller) `T` ([`Maybe::Materialized`]) is both simpler and cheaper.
    fn deserialize_maybe<T: Decode>(self) -> Result<Maybe<T>, DecodeError> {
        Ok(Maybe::Materialized(T::decode(self)?))
    }
}

pub trait SeqAccess {
    fn next_element<T>(&mut self) -> Result<Option<T>, DecodeError>
    where
        T: Decode;

    fn next_element_validate<T>(&mut self) -> Result<Option<()>, DecodeError>
    where
        T: Decode;
}

pub trait MapAccess {
    fn next_element<T>(&mut self) -> Result<Option<(&str, T)>, DecodeError>
    where
        T: Decode;

    fn next_element_validate<T>(&mut self) -> Result<Option<()>, DecodeError>
    where
        T: Decode;

    /// Advance to the next entry and return its key, or `None` at the end. The
    /// value must then be read with exactly one `next_value` / `next_value_visit`
    /// call before the following `next_key`.
    ///
    /// Unlike [`MapAccess::next_element`], this lets the caller pick the value's
    /// target type *after* seeing the key — so a value can be decoded directly
    /// from the underlying deserializer (e.g. deferred into [`super::Maybe`])
    /// instead of being materialized into a [`Value`] first.
    fn next_key(&mut self) -> Result<Option<&str>, DecodeError>;

    /// Decode the value of the entry whose key was just returned by [`MapAccess::next_key`].
    fn next_value<T: Decode>(&mut self) -> Result<T, DecodeError>;

    /// Run `visitor` over the deserializer of the current entry's value.
    fn next_value_visit<V: Visitor>(&mut self, visitor: V) -> Result<V::Value, DecodeError>;
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
        value::dispatch_ref(self, value)
    }

    fn visit_value_owned(self, value: Value) -> Result<Self::Value, DecodeError> {
        value::dispatch_owned(self, value)
    }
}
