use std::collections::BTreeMap;

use super::types::*;

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
    fn fetch_uleb(&mut self) -> anyhow::Result<num_bigint::BigUint> {
        let mut res = num_bigint::BigUint::ZERO;
        let mut off = 0u64;
        loop {
            if self.0.is_empty() {
                anyhow::bail!("unterminated uleb")
            }

            let byte = self.0[0];
            self.0 = &self.0[1..];

            res += num_bigint::BigUint::from(byte & 0x7f) << off;

            if byte & 0x80 == 0 {
                if byte == 0 && off != 0 {
                    anyhow::bail!("most significant octet can not be zero");
                }
                return Ok(res);
            }

            off = match off.checked_add(7) {
                Some(off) => off,
                None => {
                    anyhow::bail!("number is too big");
                }
            };
        }
    }

    fn fetch_slice(&mut self, le: usize) -> anyhow::Result<&[u8]> {
        if self.0.len() < le {
            anyhow::bail!("invalid size")
        }

        let ret = &self.0[..le];

        self.0 = &self.0[le..];

        Ok(ret)
    }

    fn map_to_size(size: &num_bigint::BigUint) -> anyhow::Result<usize> {
        if size.bits() > 32 {
            Err(anyhow::anyhow!(
                "container size is too large {}>32",
                size.bits()
            ))
        } else {
            Ok(size.to_u32_digits().first().cloned().unwrap_or(0) as usize)
        }
    }

    fn fetch_val(&mut self) -> anyhow::Result<Value> {
        let mut val = self.fetch_uleb()?;

        let val_least_byte = (val.iter_u32_digits().next().unwrap_or(0) & (u8::MAX as u32)) as u8;
        let typ = val_least_byte & (((1 << BITS_IN_TYPE) - 1) as u8);

        val >>= BITS_IN_TYPE;

        match typ {
            TYPE_SPECIAL => {
                if val.bits() > 8 - BITS_IN_TYPE as u64 {
                    anyhow::bail!("invalid special value {}", val << BITS_IN_TYPE)
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
                    x => Err(anyhow::anyhow!(
                        "invalid special {x}, full={}",
                        val << BITS_IN_TYPE
                    )),
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

                let mut ret = BTreeMap::new();

                for _i in 0..full_size {
                    let str_size = self.fetch_uleb()?;
                    let str_size = Self::map_to_size(&str_size)?;

                    let slice = self.fetch_slice(str_size)?;
                    let as_str = std::str::from_utf8(slice)?.to_owned();

                    if let Some((k, _)) = ret.last_key_value() {
                        if k >= &as_str {
                            anyhow::bail!("invalid calldata map ordering old=`{k}` new=`{as_str}`")
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
            v => Err(anyhow::anyhow!("invalid type {v}")),
        }
    }
}

pub fn decode(data: &[u8]) -> anyhow::Result<Value> {
    let mut parser = Parser(data);

    let ret = parser.fetch_val()?;

    if !parser.0.is_empty() {
        anyhow::bail!("input is partially unparsed")
    }

    Ok(ret)
}

fn append_uleb(to: &mut Vec<u8>, mut num: num_bigint::BigUint) {
    if num == num_bigint::BigUint::ZERO {
        to.push(0);

        return;
    }

    loop {
        let mut cur = (num.iter_u32_digits().next().unwrap_or(0) & 0xff) as u8;
        num >>= 7;
        let has_next = num != num_bigint::BigUint::ZERO;

        if has_next {
            cur |= 0x80;
        }

        to.push(cur);

        if !has_next {
            break;
        }
    }
}

pub fn encode_to(to: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => to.push(SPECIAL_NULL),
        Value::Bool(false) => to.push(SPECIAL_FALSE),
        Value::Bool(true) => to.push(SPECIAL_TRUE),
        Value::Address(address) => {
            to.push(SPECIAL_ADDR);
            to.extend_from_slice(&address.0);
        }
        Value::Str(data) => {
            let mut size = num_bigint::BigUint::from(data.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_STR; // same as |

            append_uleb(to, size);

            to.extend_from_slice(data.as_bytes());
        }
        Value::Bytes(data) => {
            let mut size = num_bigint::BigUint::from(data.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_BYTES; // same as |

            append_uleb(to, size);

            to.extend_from_slice(data);
        }
        Value::Number(big_int) => {
            if big_int.sign() == num_bigint::Sign::Minus {
                let mut mag = big_int.magnitude().clone();
                mag -= 1u32;

                mag <<= BITS_IN_TYPE;
                mag += TYPE_NINT; // same as |

                append_uleb(to, mag);
            } else {
                let mut mag = big_int.magnitude().clone();
                mag <<= BITS_IN_TYPE;
                mag += TYPE_PINT; // same as |

                append_uleb(to, mag);
            }
        }
        Value::Map(values) => {
            let mut size = num_bigint::BigUint::from(values.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_MAP; // same as |

            append_uleb(to, size);

            for (k, v) in values {
                append_uleb(to, num_bigint::BigUint::from(k.len()));
                to.extend(k.as_bytes());
                encode_to(to, v);
            }
        }
        Value::Array(values) => {
            let mut size = num_bigint::BigUint::from(values.len());
            size <<= BITS_IN_TYPE;
            size += TYPE_ARR; // same as |

            append_uleb(to, size);

            for x in values {
                encode_to(to, x);
            }
        }
    }
}

pub fn encode(value: &Value) -> Vec<u8> {
    let mut ret = Vec::new();

    encode_to(&mut ret, value);

    ret
}
