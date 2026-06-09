//! [`BinaryDeserializer`] — reads the calldata wire format directly.

use crate::consts::*;
use crate::{Address, BinDecodeError};

use super::{Decode, DecodeError, Deserializer, MapAccess, SeqAccess, Visitor};

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
