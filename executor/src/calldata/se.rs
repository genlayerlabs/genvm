use serde::ser::Error as _;
use std::collections::BTreeMap;

use super::error::*;
use super::types::*;

type Result<T> = core::result::Result<T, Error>;

pub struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Value> {
        Ok(Value::Bool(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    fn serialize_i64(self, value: i64) -> Result<Value> {
        Ok(Value::Number(value.into()))
    }

    fn serialize_i128(self, value: i128) -> Result<Value> {
        Ok(Value::Number(value.into()))
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Value> {
        Ok(Value::Number(value.into()))
    }

    fn serialize_u128(self, value: u128) -> Result<Value> {
        Ok(Value::Number(value.into()))
    }

    #[inline]
    fn serialize_f32(self, _float: f32) -> Result<Value> {
        Err(Error::custom("floats are not supported"))
    }

    #[inline]
    fn serialize_f64(self, _float: f64) -> Result<Value> {
        Err(Error::custom("floats are not supported"))
    }

    #[inline]
    fn serialize_char(self, _value: char) -> Result<Value> {
        Err(Error::custom("chars are not supported"))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Value> {
        Ok(Value::Str(value.to_owned()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
        Ok(Value::Bytes(value.to_owned()))
    }

    #[inline]
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        let mut values = BTreeMap::new();
        values.insert(String::from(variant), super::to_value(value)?);
        Ok(Value::Map(values))
    }

    #[inline]
    fn serialize_none(self) -> Result<Value> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeTupleVariant {
            name: String::from(variant),
            vec: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap {
            map: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStructVariant {
            name: String::from(variant),
            map: BTreeMap::new(),
        })
    }

    fn collect_str<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + std::fmt::Display,
    {
        Ok(Value::Str(value.to_string()))
    }
}

pub struct SerializeVec {
    vec: Vec<Value>,
}

pub struct SerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
}

pub struct SerializeMap {
    map: BTreeMap<String, Value>,
    next_key: Option<String>,
}

pub struct SerializeStructVariant {
    name: String,
    map: BTreeMap<String, Value>,
}

impl serde::ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        self.vec.push(super::to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.vec))
    }
}

impl serde::ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        self.vec.push(super::to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut object = BTreeMap::new();

        object.insert(self.name, Value::Array(self.vec));

        Ok(Value::Map(object))
    }
}

impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        self.next_key = Some(key.serialize(MapKeySerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        let key = self.next_key.take();
        // Panic because this indicates a bug in the program rather than an
        // expected failure.
        let key = key.expect("serialize_value called before serialize_key");
        self.map.insert(key, super::to_value(value)?);
        Ok(())

    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.map))
    }
}

struct MapKeySerializer;

impl serde::Serializer for MapKeySerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = serde::ser::Impossible<String, Error>;
    type SerializeTuple = serde::ser::Impossible<String, Error>;
    type SerializeTupleStruct = serde::ser::Impossible<String, Error>;
    type SerializeTupleVariant = serde::ser::Impossible<String, Error>;
    type SerializeMap = serde::ser::Impossible<String, Error>;
    type SerializeStruct = serde::ser::Impossible<String, Error>;
    type SerializeStructVariant = serde::ser::Impossible<String, Error>;

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<String> {
        Ok(variant.to_owned())
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<String>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_bool(self, _value: bool) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_i8(self, _value: i8) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_i16(self, _value: i16) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_i32(self, _value: i32) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_i64(self, _value: i64) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_i128(self, _value: i128) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_u8(self, _value: u8) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_u16(self, _value: u16) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_u32(self, _value: u32) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_u64(self, _value: u64) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_u128(self, _value: u128) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_f32(self, _value: f32) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_f64(self, _value: f64) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    #[inline]
    fn serialize_char(self, _value: char) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<String> {
        Ok(value.to_owned())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_unit(self) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_none(self) -> Result<String> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_some<T>(self, _value: &T) -> Result<String>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::custom("key can be only a string"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::custom("key can be only a string"))
    }

    fn collect_str<T>(self, value: &T) -> Result<String>
    where
        T: ?Sized + std::fmt::Display,
    {
        Ok(value.to_string())
    }
}

impl serde::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        serde::ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Value> {
        serde::ser::SerializeMap::end(self)
    }
}

impl serde::ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        self.map.insert(String::from(key), super::to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut object = BTreeMap::new();

        object.insert(self.name, Value::Map(self.map));

        Ok(Value::Map(object))
    }
}
