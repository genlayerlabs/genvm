use std::collections::BTreeMap;

use super::consts::*;
use super::types::*;

#[derive(Debug)]
pub enum BinDecodeError {
    UnterminatedUleb,
    InvalidUlebEncoding,
    NumberTooBig,
    UnexpectedEnd { expected: usize, available: usize },
    ContainerSizeTooLarge { bits: u64 },
    InvalidSpecialValue { value: u8 },
    InvalidMapOrdering { prev: String, current: String },
    InvalidTag(u8),
    InvalidUtf8(std::str::Utf8Error),
    MaxDepthExceeded,
}

impl std::fmt::Display for BinDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinDecodeError::UnterminatedUleb => write!(f, "unterminated uleb"),
            BinDecodeError::InvalidUlebEncoding => {
                write!(f, "most significant octet cannot be zero")
            }
            BinDecodeError::NumberTooBig => write!(f, "number is too big"),
            BinDecodeError::UnexpectedEnd {
                expected,
                available,
            } => {
                write!(
                    f,
                    "unexpected end: expected {expected} bytes, got {available}"
                )
            }
            BinDecodeError::ContainerSizeTooLarge { bits } => {
                write!(f, "container size is too large: {bits} > 32 bits")
            }
            BinDecodeError::InvalidSpecialValue { value } => {
                write!(f, "invalid special value: {value}")
            }
            BinDecodeError::InvalidMapOrdering { prev, current } => {
                write!(f, "invalid calldata map ordering: `{prev}` >= `{current}`")
            }
            BinDecodeError::InvalidTag(type_tag) => {
                write!(f, "invalid type tag: {type_tag}")
            }
            BinDecodeError::InvalidUtf8(e) => write!(f, "invalid utf8: {e}"),
            BinDecodeError::MaxDepthExceeded => write!(f, "exceeded maximum container depth"),
        }
    }
}

impl std::error::Error for BinDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BinDecodeError::InvalidUtf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for BinDecodeError {
    fn from(e: std::str::Utf8Error) -> Self {
        BinDecodeError::InvalidUtf8(e)
    }
}

#[derive(Clone, Copy)]
struct Parser<'a>(&'a [u8]);

impl Parser<'_> {
    fn fetch_uleb(&mut self) -> Result<num_bigint::BigUint, BinDecodeError> {
        let mut res = num_bigint::BigUint::ZERO;
        let mut off = 0u64;
        loop {
            if self.0.is_empty() {
                return Err(BinDecodeError::UnterminatedUleb);
            }

            let byte = self.0[0];
            self.0 = &self.0[1..];

            res += num_bigint::BigUint::from(byte & 0x7f) << off;

            if byte & 0x80 == 0 {
                if byte == 0 && off != 0 {
                    return Err(BinDecodeError::InvalidUlebEncoding);
                }
                return Ok(res);
            }

            off = match off.checked_add(7) {
                Some(off) => off,
                None => {
                    return Err(BinDecodeError::NumberTooBig);
                }
            };
        }
    }

    fn fetch_slice(&mut self, expected: usize) -> Result<&[u8], BinDecodeError> {
        if self.0.len() < expected {
            return Err(BinDecodeError::UnexpectedEnd {
                expected,
                available: self.0.len(),
            });
        }

        let ret = &self.0[..expected];

        self.0 = &self.0[expected..];

        Ok(ret)
    }

    fn map_to_size(size: &num_bigint::BigUint) -> Result<usize, BinDecodeError> {
        if size.bits() > 32 {
            Err(BinDecodeError::ContainerSizeTooLarge { bits: size.bits() })
        } else {
            Ok(size.to_u32_digits().first().cloned().unwrap_or(0) as usize)
        }
    }

    fn fetch_val(&mut self, opts: &Options) -> Result<Value, BinDecodeError> {
        enum Frame {
            Array {
                collected: Vec<Value>,
                remaining: usize,
            },
            Map {
                collected: BTreeMap<String, Value>,
                remaining: usize,
                current_key: String,
            },
        }

        let mut stack: Vec<Frame> = Vec::new();

        'parse: loop {
            let mut val = self.fetch_uleb()?;

            let val_least_byte =
                (val.iter_u32_digits().next().unwrap_or(0) & (u8::MAX as u32)) as u8;
            let typ = val_least_byte & (((1 << BITS_IN_TYPE) - 1) as u8);

            val >>= BITS_IN_TYPE;

            let mut completed = match typ {
                TYPE_SPECIAL => {
                    if val.bits() > 8 - BITS_IN_TYPE as u64 {
                        return Err(BinDecodeError::InvalidSpecialValue {
                            value: val_least_byte,
                        });
                    }
                    match val_least_byte {
                        SPECIAL_NULL => Value::Null,
                        SPECIAL_TRUE => Value::Bool(true),
                        SPECIAL_FALSE => Value::Bool(false),
                        SPECIAL_ADDR => {
                            let addr_slice = self.fetch_slice(ADDRESS_SIZE)?;

                            let mut addr = [0; ADDRESS_SIZE];
                            addr.copy_from_slice(addr_slice);

                            Value::Address(Address(addr))
                        }
                        x => return Err(BinDecodeError::InvalidSpecialValue { value: x }),
                    }
                }
                TYPE_BYTES => {
                    let full_size = Self::map_to_size(&val)?;
                    let slice = self.fetch_slice(full_size)?;

                    Value::Bytes(Vec::from(slice))
                }
                TYPE_STR => {
                    let full_size = Self::map_to_size(&val)?;
                    let slice = self.fetch_slice(full_size)?;

                    let as_str = std::str::from_utf8(slice)?;

                    Value::Str(String::from(as_str))
                }
                TYPE_PINT => Value::Number(num_bigint::BigInt::from_biguint(
                    num_bigint::Sign::Plus,
                    val,
                )),
                TYPE_NINT => {
                    val += 1u32;

                    Value::Number(num_bigint::BigInt::from_biguint(
                        num_bigint::Sign::Minus,
                        val,
                    ))
                }
                TYPE_ARR => {
                    if stack.len() >= opts.max_depth {
                        return Err(BinDecodeError::MaxDepthExceeded);
                    }

                    let full_size = Self::map_to_size(&val)?;
                    if self.0.len() < full_size {
                        return Err(BinDecodeError::UnexpectedEnd {
                            expected: full_size,
                            available: self.0.len(),
                        });
                    }
                    if full_size == 0 {
                        Value::Array(Vec::new())
                    } else {
                        stack.push(Frame::Array {
                            collected: Vec::with_capacity(full_size),
                            remaining: full_size,
                        });
                        continue 'parse;
                    }
                }
                TYPE_MAP => {
                    if stack.len() >= opts.max_depth {
                        return Err(BinDecodeError::MaxDepthExceeded);
                    }

                    let full_size = Self::map_to_size(&val)?;
                    if self.0.len() < full_size.saturating_mul(2) {
                        return Err(BinDecodeError::UnexpectedEnd {
                            expected: full_size.saturating_mul(2),
                            available: self.0.len(),
                        });
                    }
                    if full_size == 0 {
                        Value::Map(BTreeMap::new())
                    } else {
                        let str_size = self.fetch_uleb()?;
                        let str_size = Self::map_to_size(&str_size)?;
                        let slice = self.fetch_slice(str_size)?;
                        let current_key = std::str::from_utf8(slice)?.to_owned();

                        stack.push(Frame::Map {
                            collected: BTreeMap::new(),
                            remaining: full_size,
                            current_key,
                        });
                        continue 'parse;
                    }
                }
                v => return Err(BinDecodeError::InvalidTag(v)),
            };

            loop {
                let frame = match stack.last_mut() {
                    None => return Ok(completed),
                    Some(frame) => frame,
                };

                match frame {
                    Frame::Array {
                        collected,
                        remaining,
                    } => {
                        collected.push(completed);
                        *remaining -= 1;
                        if *remaining > 0 {
                            continue 'parse;
                        }
                        completed = Value::Array(std::mem::take(collected));
                        stack.pop();
                    }
                    Frame::Map {
                        collected,
                        remaining,
                        current_key,
                    } => {
                        let key = std::mem::take(current_key);
                        collected.insert(key, completed);
                        *remaining -= 1;
                        if *remaining > 0 {
                            let str_size = self.fetch_uleb()?;
                            let str_size = Self::map_to_size(&str_size)?;
                            let slice = self.fetch_slice(str_size)?;
                            let new_key = std::str::from_utf8(slice)?.to_owned();

                            if let Some((k, _)) = collected.last_key_value()
                                && k >= &new_key
                            {
                                return Err(BinDecodeError::InvalidMapOrdering {
                                    prev: k.clone(),
                                    current: new_key,
                                });
                            }

                            *current_key = new_key;
                            continue 'parse;
                        }
                        completed = Value::Map(std::mem::take(collected));
                        stack.pop();
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub max_depth: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_depth: 128 }
    }
}

pub fn decode_with(data: &[u8], opts: &Options) -> Result<Value, BinDecodeError> {
    let mut parser = Parser(data);
    parser.fetch_val(opts)
}

pub fn decode(data: &[u8]) -> Result<Value, BinDecodeError> {
    let mut parser = Parser(data);

    let ret = parser.fetch_val(&Default::default())?;

    if !parser.0.is_empty() {
        return Err(BinDecodeError::UnexpectedEnd {
            expected: 0,
            available: parser.0.len(),
        });
    }

    Ok(ret)
}

pub use super::encoder::{Encoder, Writer};

pub fn encode_to<W: Writer>(enc: &mut Encoder<W>, value: &Value) -> Result<(), W::Error> {
    enum Item<'a> {
        Value(&'a Value),
        MapKey(&'a str),
    }

    let mut stack: Vec<Item> = vec![Item::Value(value)];

    while let Some(item) = stack.pop() {
        match item {
            Item::MapKey(key) => {
                enc.push_map_k(key)?;
            }
            Item::Value(value) => match value {
                Value::Null => enc.push_null()?,
                Value::Bool(v) => enc.push_bool(*v)?,
                Value::Address(address) => enc.push_address(address)?,
                Value::Str(data) => enc.push_str(data)?,
                Value::Bytes(data) => enc.push_bytes(data)?,
                Value::Number(big_int) => enc.push_bigint(big_int)?,
                Value::Map(values) => {
                    enc.start_map(values.len() as u64)?;

                    for (k, v) in values.iter().rev() {
                        stack.push(Item::Value(v));
                        stack.push(Item::MapKey(k));
                    }
                }
                Value::Array(values) => {
                    enc.start_array(values.len() as u64)?;

                    for x in values.iter().rev() {
                        stack.push(Item::Value(x));
                    }
                }
            },
        }
    }

    Ok(())
}

impl Writer for Vec<u8> {
    type Error = std::convert::Infallible;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.extend_from_slice(data);
        Ok(())
    }
}

impl Writer for &mut Vec<u8> {
    type Error = std::convert::Infallible;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.extend_from_slice(data);
        Ok(())
    }
}

pub fn encode(value: &Value) -> Vec<u8> {
    let mut ret = Vec::new();
    let mut enc = Encoder::new(&mut ret);

    match encode_to(&mut enc, value) {
        Ok(()) => {}
        Err(e) => match e {},
    }

    ret
}
