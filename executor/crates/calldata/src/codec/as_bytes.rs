use crate::codec::DecodeError;
use crate::{Encoder, Value, Writer};

/// Trait implemented by types that can be encoded/decoded as raw bytes.
pub trait Codec: Sized {
    fn encode_bytes<W: Writer>(&self, enc: &mut Encoder<W>) -> Result<(), W::Error>;
    fn decode_bytes(val: Value) -> Result<Self, DecodeError>;
}

impl Codec for Vec<u8> {
    fn encode_bytes<W: Writer>(&self, enc: &mut Encoder<W>) -> Result<(), W::Error> {
        enc.push_bytes(self)
    }

    fn decode_bytes(val: Value) -> Result<Self, DecodeError> {
        match val {
            Value::Bytes(b) => Ok(b),
            _ => Err(DecodeError::Unexpected("expected bytes")),
        }
    }
}

impl<const N: usize> Codec for [u8; N] {
    fn encode_bytes<W: Writer>(&self, enc: &mut Encoder<W>) -> Result<(), W::Error> {
        enc.push_bytes(self.as_slice())
    }

    fn decode_bytes(val: Value) -> Result<Self, DecodeError> {
        match val {
            Value::Bytes(b) => b
                .try_into()
                .map_err(|_| DecodeError::Unexpected("expected exact byte length")),
            _ => Err(DecodeError::Unexpected("expected bytes")),
        }
    }
}

impl Codec for Option<Vec<u8>> {
    fn encode_bytes<W: Writer>(&self, enc: &mut Encoder<W>) -> Result<(), W::Error> {
        match self {
            Some(v) => enc.push_bytes(v),
            None => enc.push_null(),
        }
    }

    fn decode_bytes(val: Value) -> Result<Self, DecodeError> {
        match val {
            Value::Null => Ok(None),
            Value::Bytes(b) => Ok(Some(b)),
            _ => Err(DecodeError::Unexpected("expected bytes or null")),
        }
    }
}

pub fn serialize<W: Writer>(val: &impl Codec, enc: &mut Encoder<W>) -> Result<(), W::Error> {
    val.encode_bytes(enc)
}

pub fn deserialize<T: Codec>(val: Value) -> Result<T, DecodeError> {
    T::decode_bytes(val)
}
