//! Deferred (lazy) decoding.
//!
//! [`Maybe<T>`] lets a field decoded from the wire be decoded later: at decode
//! time the value is only *validated* as `T` (an `O(depth)` structural walk, see
//! [`Decode::validate`]) and the well-formed payload is retained compactly as the
//! original wire bytes ([`Maybe::Checked`]). [`Maybe::materialize`] then pays the
//! full `T::decode` cost on demand. Decoding from a source that is not byte-backed
//! (an in-memory [`Value`]) has nothing to defer, so it produces an eager
//! [`Maybe::Materialized`].
//!
//! The budget caveat: validation is bounded per call, but a fresh
//! [`BinaryDeserializer`] is used when materializing, so the depth/size limits
//! reset across a `Maybe` boundary. Each [`Maybe::materialize`] is independently
//! bounded; the *total* resident size if every `Maybe` is materialized is not.

use crate::codec::Encode;
use crate::{Encoder, Value, Writer};

use super::{BinaryDeserializer, Decode, DecodeError, Deserializer, ValueDeserializer};

/// Validated-but-unparsed calldata: a single well-formed value kept as raw wire bytes.
#[derive(Debug, Clone)]
pub struct Raw(pub bytes::Bytes);

/// A value that is either materialized eagerly, or validated-but-deferred as raw bytes.
#[derive(Debug, Clone)]
pub enum Maybe<T> {
    Materialized(T),
    Checked(Raw),
}

impl Raw {
    /// Decode the captured wire bytes as `T`.
    pub fn decode_as<T: Decode>(&self) -> Result<T, DecodeError> {
        let mut de = BinaryDeserializer::new(&self.0);
        let value = T::decode(&mut de)?;
        if !de.is_empty() {
            return Err(crate::BinDecodeError::UnexpectedEnd {
                expected: 0,
                available: de.remaining(),
            }
            .into());
        }
        Ok(value)
    }

    /// Capture the canonical wire encoding of a [`Value`].
    pub fn from_value(value: &Value) -> Self {
        Raw(crate::encode(value).into())
    }
}

impl<W: Writer> Encode<W> for Raw {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        // The captured bytes are already a complete encoded value — pass through.
        enc.write_raw(&self.0)
    }
}

impl<W, T> Encode<W> for Maybe<T>
where
    W: Writer,
    T: Encode<W, Error = W::Error>,
{
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        match self {
            Maybe::Materialized(value) => value.encode(enc),
            Maybe::Checked(raw) => enc.write_raw(&raw.0),
        }
    }
}

impl Decode for Raw {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        match deserializer.deserialize_maybe::<Value>()? {
            Maybe::Checked(raw) => Ok(raw),
            Maybe::Materialized(value) => Ok(Raw::from_value(&value)),
        }
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        Value::validate(deserializer)
    }
}

impl<T: Decode> Maybe<T> {
    /// Materialize the deferred value into `T`, paying the full decode cost now.
    pub fn materialize(self) -> Result<T, DecodeError> {
        match self {
            Maybe::Materialized(value) => Ok(value),
            Maybe::Checked(raw) => raw.decode_as::<T>(),
        }
    }
}

impl Maybe<Value> {
    /// Decode a deferred [`Value`] into some other type `U`.
    ///
    /// On the byte-backed path ([`Maybe::Checked`]) this decodes `U` straight
    /// from the retained wire bytes, so no [`Value`] is ever materialized. When
    /// the value was already materialized it is decoded in-memory. This is what
    /// lets an internally tagged enum's ambiguously-typed field (buffered as
    /// `Maybe<Value>` before the tag is known) be turned into the exact
    /// per-variant type without a `Value` round-trip.
    pub fn decode_into<U: Decode>(self) -> Result<U, DecodeError> {
        match self {
            Maybe::Checked(raw) => raw.decode_as::<U>(),
            Maybe::Materialized(value) => U::decode(ValueDeserializer(value)),
        }
    }
}

impl<T: Decode> Decode for Maybe<T> {
    fn decode<D: Deserializer>(deserializer: D) -> Result<Self, DecodeError> {
        deserializer.deserialize_maybe::<T>()
    }

    fn validate<D: Deserializer>(deserializer: D) -> Result<(), DecodeError> {
        T::validate(deserializer)
    }
}
