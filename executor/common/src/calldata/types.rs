use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const ADDRESS_SIZE: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Address(pub(super) [u8; ADDRESS_SIZE]);

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("addr#{}", hex::encode(self.0)))
    }
}

impl Address {
    pub const SIZE: u32 = 20;

    pub const fn from(raw: [u8; ADDRESS_SIZE]) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> [u8; ADDRESS_SIZE] {
        self.0
    }

    pub fn ref_mut(&mut self) -> &mut [u8; ADDRESS_SIZE] {
        &mut self.0
    }

    pub const fn zero() -> Self {
        Self([0; 20])
    }

    pub const fn len() -> usize {
        20
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Address(Address),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    Number(num_bigint::BigInt),
    Map(BTreeMap<String, Value>),
    Array(Vec<Value>),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Address(arg0) => f.write_fmt(format_args!("{arg0:?}")),
            Self::Bool(true) => f.write_str("true"),
            Self::Bool(false) => f.write_str("false"),
            Self::Str(str) => f.write_fmt(format_args!("{str:?}")),
            Self::Bytes(bytes) => {
                f.write_str("b#")?;
                if bytes.len() > 64 {
                    f.write_str(&hex::encode(&bytes[..32]))?;
                    f.write_str("...")?;
                    f.write_str(&hex::encode(&bytes[bytes.len() - 32..]))?;
                } else {
                    f.write_str(&hex::encode(bytes))?;
                }
                Ok(())
            }
            Self::Number(num) => f.write_fmt(format_args!("{num:}")),
            Self::Map(map) => {
                f.write_str("{")?;
                let mut first = true;
                for (k, v) in map {
                    if !first {
                        f.write_str(",")?;
                    }

                    f.write_fmt(format_args!("{k:?}"))?;
                    f.write_str(":")?;
                    v.fmt(f)?;

                    first = false;
                }
                f.write_str("}")?;
                Ok(())
            }
            Self::Array(arr) => {
                f.write_str("[")?;
                let mut first = true;
                for v in arr {
                    if !first {
                        f.write_str(",")?;
                    }

                    v.fmt(f)?;

                    first = false;
                }
                f.write_str("]")?;
                Ok(())
            }
        }
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Str(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: std::string::String) -> Self {
        Value::Str(v)
    }
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}
