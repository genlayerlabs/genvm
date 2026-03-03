use std::collections::BTreeMap;

use super::types::*;

#[derive(Debug)]
pub enum DecodeError {
    UnterminatedUleb,
    InvalidUlebEncoding,
    NumberTooBig,
    UnexpectedEnd { expected: usize, available: usize },
    ContainerSizeTooLarge { bits: u64 },
    InvalidSpecialValue { value: u8 },
    InvalidMapOrdering { prev: String, current: String },
    InvalidType { type_tag: u8 },
    TrailingData { remaining: usize },
    InvalidUtf8(std::str::Utf8Error),
    Custom(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnterminatedUleb => write!(f, "unterminated uleb"),
            DecodeError::InvalidUlebEncoding => {
                write!(f, "most significant octet cannot be zero")
            }
            DecodeError::NumberTooBig => write!(f, "number is too big"),
            DecodeError::UnexpectedEnd {
                expected,
                available,
            } => {
                write!(
                    f,
                    "unexpected end: expected {expected} bytes, got {available}"
                )
            }
            DecodeError::ContainerSizeTooLarge { bits } => {
                write!(f, "container size is too large: {bits} > 32 bits")
            }
            DecodeError::InvalidSpecialValue { value } => {
                write!(f, "invalid special value: {value}")
            }
            DecodeError::InvalidMapOrdering { prev, current } => {
                write!(f, "invalid calldata map ordering: `{prev}` >= `{current}`")
            }
            DecodeError::InvalidType { type_tag } => {
                write!(f, "invalid type tag: {type_tag}")
            }
            DecodeError::TrailingData { remaining } => {
                write!(
                    f,
                    "input is partially unparsed: {remaining} bytes remaining"
                )
            }
            DecodeError::InvalidUtf8(e) => write!(f, "invalid utf8: {e}"),
            DecodeError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::InvalidUtf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for DecodeError {
    fn from(e: std::str::Utf8Error) -> Self {
        DecodeError::InvalidUtf8(e)
    }
}

#[derive(Clone, Copy)]
struct Parser<'a>(&'a [u8]);

const BITS_IN_TYPE: usize = 3;

const TYPE_SPECIAL: u8 = 0;
const TYPE_PINT: u8 = 1;
const TYPE_NINT: u8 = 2;
const TYPE_BYTES: u8 = 3;
const TYPE_STR: u8 = 4;
const TYPE_ARR: u8 = 5;
const TYPE_MAP: u8 = 6;

const SPECIAL_NULL: u8 = (0 << BITS_IN_TYPE) | TYPE_SPECIAL;
const SPECIAL_FALSE: u8 = (1 << BITS_IN_TYPE) | TYPE_SPECIAL;
const SPECIAL_TRUE: u8 = (2 << BITS_IN_TYPE) | TYPE_SPECIAL;
const SPECIAL_ADDR: u8 = (3 << BITS_IN_TYPE) | TYPE_SPECIAL;

impl Parser<'_> {
    fn fetch_uleb(&mut self) -> Result<num_bigint::BigUint, DecodeError> {
        let mut res = num_bigint::BigUint::ZERO;
        let mut off = 0u64;
        loop {
            if self.0.is_empty() {
                return Err(DecodeError::UnterminatedUleb);
            }

            let byte = self.0[0];
            self.0 = &self.0[1..];

            res += num_bigint::BigUint::from(byte & 0x7f) << off;

            if byte & 0x80 == 0 {
                if byte == 0 && off != 0 {
                    return Err(DecodeError::InvalidUlebEncoding);
                }
                return Ok(res);
            }

            off = match off.checked_add(7) {
                Some(off) => off,
                None => {
                    return Err(DecodeError::NumberTooBig);
                }
            };
        }
    }

    fn fetch_slice(&mut self, expected: usize) -> Result<&[u8], DecodeError> {
        if self.0.len() < expected {
            return Err(DecodeError::UnexpectedEnd {
                expected,
                available: self.0.len(),
            });
        }

        let ret = &self.0[..expected];

        self.0 = &self.0[expected..];

        Ok(ret)
    }

    fn map_to_size(size: &num_bigint::BigUint) -> Result<usize, DecodeError> {
        if size.bits() > 32 {
            Err(DecodeError::ContainerSizeTooLarge { bits: size.bits() })
        } else {
            Ok(size.to_u32_digits().first().cloned().unwrap_or(0) as usize)
        }
    }

    fn fetch_val(&mut self) -> Result<Value, DecodeError> {
        let mut val = self.fetch_uleb()?;

        let val_least_byte = (val.iter_u32_digits().next().unwrap_or(0) & (u8::MAX as u32)) as u8;
        let typ = val_least_byte & (((1 << BITS_IN_TYPE) - 1) as u8);

        val >>= BITS_IN_TYPE;

        match typ {
            TYPE_SPECIAL => {
                if val.bits() > 8 - BITS_IN_TYPE as u64 {
                    return Err(DecodeError::InvalidSpecialValue {
                        value: val_least_byte,
                    });
                }
                match val_least_byte {
                    SPECIAL_NULL => Ok(Value::Null),
                    SPECIAL_TRUE => Ok(Value::Bool(true)),
                    SPECIAL_FALSE => Ok(Value::Bool(false)),
                    SPECIAL_ADDR => {
                        let addr_slice = self.fetch_slice(ADDRESS_SIZE)?;

                        let mut addr = [0; ADDRESS_SIZE];
                        addr.copy_from_slice(addr_slice);

                        Ok(Value::Address(Address(addr)))
                    }
                    x => Err(DecodeError::InvalidSpecialValue { value: x }),
                }
            }
            TYPE_BYTES => {
                let full_size = Self::map_to_size(&val)?;
                let slice = self.fetch_slice(full_size)?;

                Ok(Value::Bytes(Vec::from(slice)))
            }
            TYPE_ARR => {
                let full_size = Self::map_to_size(&val)?;
                let mut ret = Vec::new();

                for _i in 0..full_size {
                    ret.push(self.fetch_val()?);
                }

                Ok(Value::Array(ret))
            }
            TYPE_STR => {
                let full_size = Self::map_to_size(&val)?;
                let slice = self.fetch_slice(full_size)?;

                let as_str = std::str::from_utf8(slice)?;

                Ok(Value::Str(String::from(as_str)))
            }
            TYPE_MAP => {
                let full_size = Self::map_to_size(&val)?;

                let mut ret: BTreeMap<String, Value> = BTreeMap::new();

                for _i in 0..full_size {
                    let str_size = self.fetch_uleb()?;
                    let str_size = Self::map_to_size(&str_size)?;

                    let slice = self.fetch_slice(str_size)?;
                    let as_str = std::str::from_utf8(slice)?.to_owned();

                    if let Some((k, _)) = ret.last_key_value() {
                        if k >= &as_str {
                            return Err(DecodeError::InvalidMapOrdering {
                                prev: k.clone(),
                                current: as_str,
                            });
                        }
                    }

                    let val = self.fetch_val()?;

                    ret.insert(as_str, val);
                }

                Ok(Value::Map(ret))
            }
            TYPE_NINT => {
                val += 1u32;

                Ok(Value::Number(num_bigint::BigInt::from_biguint(
                    num_bigint::Sign::Minus,
                    val,
                )))
            }
            TYPE_PINT => Ok(Value::Number(num_bigint::BigInt::from_biguint(
                num_bigint::Sign::Plus,
                val,
            ))),
            v => Err(DecodeError::InvalidType { type_tag: v }),
        }
    }
}

pub fn decode(data: &[u8]) -> Result<Value, DecodeError> {
    let mut parser = Parser(data);

    let ret = parser.fetch_val()?;

    if !parser.0.is_empty() {
        return Err(DecodeError::TrailingData {
            remaining: parser.0.len(),
        });
    }

    Ok(ret)
}

fn append_uleb<A: Appender>(
    to: &mut A,
    mut num: num_bigint::BigUint,
) -> core::result::Result<(), A::Error> {
    if num == num_bigint::BigUint::ZERO {
        to.write_all(&[0])?;
        return Ok(());
    }

    loop {
        let mut cur = (num.iter_u32_digits().next().unwrap_or(0) & 0xff) as u8;
        num >>= 7;
        let has_next = num != num_bigint::BigUint::ZERO;

        if has_next {
            cur |= 0x80;
        }

        to.write_all(&[cur])?;

        if !has_next {
            return Ok(());
        }
    }
}

pub trait Appender {
    type Error;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    fn write_one(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.write_all(&[byte])
    }
}

pub fn encode_to<A: Appender>(to: &mut A, value: &Value) -> core::result::Result<(), A::Error> {
    match value {
        Value::Null => to.write_all(&[SPECIAL_NULL]),
        Value::Bool(false) => to.write_all(&[SPECIAL_FALSE]),
        Value::Bool(true) => to.write_all(&[SPECIAL_TRUE]),
        Value::Address(address) => {
            to.write_all(&[SPECIAL_ADDR])?;
            to.write_all(&address.0)
        }
        Value::Str(data) => {
            let mut size = num_bigint::BigUint::from(data.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_STR; // same as |

            append_uleb(to, size)?;
            to.write_all(data.as_bytes())
        }
        Value::Bytes(data) => {
            let mut size = num_bigint::BigUint::from(data.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_BYTES; // same as |

            append_uleb(to, size)?;
            to.write_all(data)
        }
        Value::Number(big_int) => {
            if big_int.sign() == num_bigint::Sign::Minus {
                let mut mag = big_int.magnitude().clone();
                mag -= 1u32;

                mag <<= BITS_IN_TYPE;
                mag += TYPE_NINT; // same as |

                append_uleb(to, mag)
            } else {
                let mut mag = big_int.magnitude().clone();
                mag <<= BITS_IN_TYPE;
                mag += TYPE_PINT; // same as |

                append_uleb(to, mag)
            }
        }
        Value::Map(values) => {
            let mut size = num_bigint::BigUint::from(values.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_MAP; // same as |

            append_uleb(to, size)?;

            for (k, v) in values {
                append_uleb(to, num_bigint::BigUint::from(k.len()))?;
                to.write_all(k.as_bytes())?;
                encode_to(to, v)?;
            }
            Ok(())
        }
        Value::Array(values) => {
            let mut size = num_bigint::BigUint::from(values.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_ARR; // same as |

            append_uleb(to, size)?;

            for x in values {
                encode_to(to, x)?;
            }
            Ok(())
        }
    }
}

impl Appender for Vec<u8> {
    type Error = std::convert::Infallible;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.extend_from_slice(data);
        Ok(())
    }
}

pub fn encode(value: &Value) -> Vec<u8> {
    let mut ret = Vec::new();

    match encode_to(&mut ret, value) {
        Ok(()) => ret,
        Err(e) => match e {},
    }
}
