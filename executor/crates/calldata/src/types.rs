use std::collections::BTreeMap;

pub const ADDRESS_SIZE: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Address(pub(super) [u8; ADDRESS_SIZE]);

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("addr#{}", hex::encode(self.0)))
    }
}

#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for Address {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut raw = [0u8; ADDRESS_SIZE];
        u.fill_buffer(&mut raw)?;
        Ok(Address(raw))
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

pub type Map = BTreeMap<String, Value>;

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

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Address(arg0) => f.write_fmt(format_args!("{arg0:?}")),
            Self::Bool(true) => f.write_str("true"),
            Self::Bool(false) => f.write_str("false"),
            Self::Str(str) => f.write_fmt(format_args!("{str:?}")),
            Self::Bytes(bytes) => {
                f.write_str("b#")?;
                f.write_str(&hex::encode(bytes))?;
                Ok(())
            }
            Self::Number(num) => f.write_fmt(format_args!("{num}")),
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

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<Address> for Value {
    fn from(v: Address) -> Self {
        Value::Address(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl From<num_bigint::BigInt> for Value {
    fn from(v: num_bigint::BigInt) -> Self {
        Value::Number(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Number(num_bigint::BigInt::from(v))
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Number(num_bigint::BigInt::from(v))
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Number(num_bigint::BigInt::from(v))
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Number(num_bigint::BigInt::from(v))
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(v: BTreeMap<String, Value>) -> Self {
        Value::Map(v)
    }
}

impl From<primitive_types::U256> for Value {
    fn from(v: primitive_types::U256) -> Self {
        let bytes = v.to_little_endian();
        Value::Number(num_bigint::BigInt::from_bytes_le(
            num_bigint::Sign::Plus,
            &bytes,
        ))
    }
}

#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for Value {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Self::arbitrary_depth(u, 3)
    }
}

#[cfg(feature = "arbitrary")]
impl Value {
    fn arbitrary_depth(u: &mut arbitrary::Unstructured<'_>, depth: u8) -> arbitrary::Result<Self> {
        if depth == 0 {
            return match u.int_in_range(0..=5u8)? {
                0 => Ok(Value::Null),
                1 => Ok(Value::Bool(u.arbitrary()?)),
                2 => Ok(Value::Str(u.arbitrary()?)),
                3 => {
                    if u.arbitrary()? {
                        let bytes: [u8; 32] = u.arbitrary()?;
                        let sign = if u.arbitrary()? {
                            num_bigint::Sign::Plus
                        } else {
                            num_bigint::Sign::Minus
                        };
                        let big = num_bigint::BigInt::from_bytes_le(sign, &bytes);
                        Ok(Value::Number(big))
                    } else {
                        Ok(Value::Number(num_bigint::BigInt::from(
                            u.arbitrary::<i64>()?,
                        )))
                    }
                }
                4 => Ok(Value::Bytes(u.arbitrary()?)),
                _ => Ok(Value::Address(u.arbitrary()?)),
            };
        }
        match u.int_in_range(0..=7u8)? {
            0 => Ok(Value::Null),
            1 => Ok(Value::Bool(u.arbitrary()?)),
            2 => Ok(Value::Str(u.arbitrary()?)),
            3 => Ok(Value::Number(num_bigint::BigInt::from(
                u.arbitrary::<i64>()?,
            ))),
            4 => Ok(Value::Bytes(u.arbitrary()?)),
            5 => Ok(Value::Address(u.arbitrary()?)),
            6 => {
                let len = u.int_in_range(0..=4u8)?;
                (0..len)
                    .map(|_| Self::arbitrary_depth(u, depth - 1))
                    .collect::<arbitrary::Result<Vec<_>>>()
                    .map(Value::Array)
            }
            _ => {
                let len = u.int_in_range(0..=4u8)?;
                let mut map = BTreeMap::new();
                for _ in 0..len {
                    map.insert(u.arbitrary()?, Self::arbitrary_depth(u, depth - 1)?);
                }
                Ok(Value::Map(map))
            }
        }
    }
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&num_bigint::BigInt> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_address(&self) -> Option<&Address> {
        match self {
            Value::Address(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}
