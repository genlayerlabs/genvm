use serde::de::Error as _;
use serde::de::IntoDeserializer;
use std::collections::BTreeMap;

use super::error::*;
use super::types::*;

impl<'de> serde::de::Deserialize<'de> for Value {
    #[inline]
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

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            #[inline]
            fn visit_i128<E>(self, value: i128) -> Result<Value, E> {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            fn visit_u128<E>(self, value: u128) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Number(num_bigint::BigInt::from(value)))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                // TODO: we may want to check for integer value
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::Str(value))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                serde::de::Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
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
                    if let Some(_old) = map.insert(k, v) {
                        return Err(serde::de::Error::duplicate_field("<>"));
                    }
                }

                Ok(Value::Map(map))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Value>,
}

impl SeqDeserializer {
    fn new(vec: Vec<Value>) -> Self {
        SeqDeserializer {
            iter: vec.into_iter(),
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => deserialize_with_seed(value, seed).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

fn visit_array<'de, V>(array: Vec<Value>, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    let len = array.len();
    let mut deserializer = SeqDeserializer::new(array);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(serde::de::Error::invalid_length(
            len,
            &"fewer elements in array",
        ))
    }
}

struct MapDeserializer {
    iter: <BTreeMap<String, Value> as IntoIterator>::IntoIter,
    value: Option<Value>,
}

impl MapDeserializer {
    fn new(map: BTreeMap<String, Value>) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}

struct MapKeyDeserializer {
    key: String,
}

macro_rules! deserialize_numeric_key {
    ($method:ident) => {
        deserialize_numeric_key!($method, deserialize_number);
    };

    ($method:ident, $using:ident) => {
        fn $method<V>(self, _visitor: V) -> Result<V::Value, Error>
        where
            V: serde::de::Visitor<'de>,
        {
            Err(Error::custom("only string keys are supported"))
        }
    };
}

impl<'de> serde::Deserializer<'de> for MapKeyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_str(&self.key)
    }

    deserialize_numeric_key!(deserialize_i8);
    deserialize_numeric_key!(deserialize_i16);
    deserialize_numeric_key!(deserialize_i32);
    deserialize_numeric_key!(deserialize_i64);
    deserialize_numeric_key!(deserialize_u8);
    deserialize_numeric_key!(deserialize_u16);
    deserialize_numeric_key!(deserialize_u32);
    deserialize_numeric_key!(deserialize_u64);
    deserialize_numeric_key!(deserialize_f32);
    deserialize_numeric_key!(deserialize_f64);
    deserialize_numeric_key!(deserialize_i128, do_deserialize_i128);
    deserialize_numeric_key!(deserialize_u128, do_deserialize_u128);

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.key == "true" {
            visitor.visit_bool(true)
        } else if self.key == "false" {
            visitor.visit_bool(false)
        } else {
            Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::Str(&self.key),
                &visitor,
            ))
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        // Map keys cannot be null.
        visitor.visit_some(self)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.key
            .into_deserializer()
            .deserialize_enum(name, variants, visitor)
    }

    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf unit unit_struct seq tuple tuple_struct
        map struct identifier ignored_any
    }
}

fn deserialize_with_seed<'de, T>(value: Value, seed: T) -> Result<T::Value, Error>
where
    T: serde::de::DeserializeSeed<'de>
{
    let full_name = std::any::type_name::<T::Value>();
    if full_name == "num_bigint::bigint::BigInt" {
        match value {
            Value::Number(num) => {
                let ptr = std::ptr::from_ref(&num) as *const T::Value;
                Ok(unsafe { ptr.read() })
            },
            _ => Err(serde::de::Error::custom("invalid type")),
        }
    } else if full_name == "genvm::calldata::types::Address" {
        match value {
            Value::Address(addr) => {
                let ptr = std::ptr::from_ref(&addr) as *const T::Value;
                Ok(unsafe { ptr.read() })
            },
            _ => Err(serde::de::Error::custom("invalid type")),
        }
    } else {
        seed.deserialize(value)
    }
}

impl<'de> serde::de::MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_de = MapKeyDeserializer { key };
                seed.deserialize(key_de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                deserialize_with_seed(value, seed)
            },
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

fn visit_map<'de, V>(map: BTreeMap<String, Value>, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    let len = map.len();
    let mut deserializer = MapDeserializer::new(map);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(serde::de::Error::invalid_length(
            len,
            &"fewer elements in map",
        ))
    }
}

fn visit_map_enum<'de, V>(map: BTreeMap<String, Value>, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    let mut iter = map.into_iter();
    let (variant, value) = match iter.next() {
        Some(v) => v,
        None => {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Map,
                &"map with a single key",
            ));
        }
    };
    // enums are encoded in json as maps with a single key:value pair
    if iter.next().is_some() {
        return Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Map,
            &"map with a single key",
        ));
    }

    visitor.visit_enum(EnumDeserializer {
        variant,
        value: Some(value),
    })
}

struct VariantDeserializer {
    value: Option<Value>,
}

impl<'de> serde::de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => serde::de::Deserialize::deserialize(value),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Some(Value::Array(v)) => {
                if v.is_empty() {
                    visitor.visit_unit()
                } else {
                    visit_array(v, visitor)
                }
            }
            Some(other) => Err(serde::de::Error::invalid_type(
                other.unexpected(),
                &"tuple variant",
            )),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Some(Value::Map(v)) => visit_map(v, visitor),
            Some(other) => Err(serde::de::Error::invalid_type(
                other.unexpected(),
                &"struct variant",
            )),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

struct EnumDeserializer {
    variant: String,
    value: Option<Value>,
}

impl<'de> serde::de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer), Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let visitor = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

fn visit_bigint<'de, V>(num: num_bigint::BigInt, visitor: V) -> Result<V::Value, Error>
where
    V: serde::de::Visitor<'de>,
{
    if let Ok(res) = i64::try_from(&num) {
        visitor.visit_i64(res)
    } else if let Ok(res) = u64::try_from(&num) {
        visitor.visit_u64(res)
    } else if let Ok(res) = i128::try_from(&num) {
        visitor.visit_i128(res)
    } else if let Ok(res) = u128::try_from(&num) {
        visitor.visit_u128(res)
    } else {
        Err(Error::custom("number is too big"))
    }
}

macro_rules! deserialize_number {
    ($method:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Error>
        where
            V: serde::de::Visitor<'de>,
        {
            match self {
                Value::Number(n) => visit_bigint(n, visitor),
                _ => Err(self.invalid_type(&visitor)),
            }
        }
    };
}

impl<'de> serde::Deserializer<'de> for Value {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let full_name = std::any::type_name::<V::Value>();
        match self {
            Value::Address(_a) => Err(Error(anyhow::anyhow!("unexpected address"))),
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Number(n) => visit_bigint(n, visitor),
            Value::Str(v) => visitor.visit_string(v),
            Value::Bytes(v) => visitor.visit_byte_buf(v),
            Value::Array(v) => visit_array(v, visitor),
            Value::Map(v) => visit_map(v, visitor),
        }
    }

    deserialize_number!(deserialize_i8);
    deserialize_number!(deserialize_i16);
    deserialize_number!(deserialize_i32);
    deserialize_number!(deserialize_i64);
    deserialize_number!(deserialize_i128);
    deserialize_number!(deserialize_u8);
    deserialize_number!(deserialize_u16);
    deserialize_number!(deserialize_u32);
    deserialize_number!(deserialize_u64);
    deserialize_number!(deserialize_u128);
    deserialize_number!(deserialize_f32);
    deserialize_number!(deserialize_f64);

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Map(value) => visit_map_enum(value, visitor),
            Value::Str(variant) => visitor.visit_enum(EnumDeserializer {
                variant,
                value: None,
            }),
            other => Err(serde::de::Error::invalid_type(
                other.unexpected(),
                &"string or map",
            )),
        }
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let full_name = std::any::type_name::<V::Value>();
        println!("deserialize newtype {} of name {}", full_name, _name);
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Bool(v) => visitor.visit_bool(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Str(v) => visitor.visit_string(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Bytes(v) => visitor.visit_byte_buf(v),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Array(v) => visit_array(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Map(v) => visit_map(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            Value::Array(v) => visit_array(v, visitor),
            Value::Map(v) => visit_map(v, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}

impl Value {
    #[cold]
    fn invalid_type<E>(&self, exp: &dyn serde::de::Expected) -> E
    where
        E: serde::de::Error,
    {
        serde::de::Error::invalid_type(self.unexpected(), exp)
    }

    #[cold]
    fn unexpected(&self) -> serde::de::Unexpected {
        use serde::de::Unexpected;

        match self {
            Value::Null => Unexpected::Unit,
            Value::Bool(b) => Unexpected::Bool(*b),
            Value::Number(_) => Unexpected::Other("bigint"),
            Value::Address(_) => Unexpected::Other("address"),
            Value::Str(s) => Unexpected::Str(s),
            Value::Bytes(s) => Unexpected::Bytes(s),
            Value::Array(_) => Unexpected::Seq,
            Value::Map(_) => Unexpected::Map,
        }
    }
}
