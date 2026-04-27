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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Null,
    Bool,
    Address,
    Number,
    Str,
    Bytes,
    Array,
    Map,
}

impl std::fmt::Display for ValueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ValueKind::Null => "null",
            ValueKind::Bool => "bool",
            ValueKind::Address => "address",
            ValueKind::Number => "number",
            ValueKind::Str => "str",
            ValueKind::Bytes => "bytes",
            ValueKind::Array => "array",
            ValueKind::Map => "map",
        })
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

    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Null => ValueKind::Null,
            Value::Bool(_) => ValueKind::Bool,
            Value::Address(_) => ValueKind::Address,
            Value::Number(_) => ValueKind::Number,
            Value::Str(_) => ValueKind::Str,
            Value::Bytes(_) => ValueKind::Bytes,
            Value::Array(_) => ValueKind::Array,
            Value::Map(_) => ValueKind::Map,
        }
    }
}

impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Bytes(b) => serializer.serialize_bytes(b),
            Value::Str(s) => serializer.serialize_str(s),
            Value::Array(v) => v.serialize(serializer),
            Value::Map(m) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Value::Address(addr) => addr.serialize(serializer),
            Value::Number(num) => {
                if let Ok(num) = num.try_into() {
                    let num: i64 = num;
                    serializer.serialize_i64(num)
                } else {
                    num.serialize(serializer)
                }
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any valid calldata value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            fn visit_i128<E>(self, value: i128) -> Result<Value, E> {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            fn visit_u128<E>(self, value: u128) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(String::from(value))
            }

            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::Str(value))
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                serde::Deserialize::deserialize(deserializer)
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Array(vec))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut map = BTreeMap::new();
                while let Some((k, v)) = visitor.next_entry()? {
                    map.insert(k, v);
                }
                Ok(Value::Map(map))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Bytes(v))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Bytes(v.to_owned()))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}
