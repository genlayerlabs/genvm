use std::sync::Arc;

use crate::{Address, Encoder, Value, Writer};

pub trait Encode<W>
where
    W: Writer,
{
    type Error;
    fn encode(&self, encoder: &mut Encoder<W>) -> Result<(), Self::Error>;
}

impl<W: Writer> Encode<W> for Value {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enum Item<'a> {
            Value(&'a Value),
            MapKey(&'a str),
        }

        let mut stack: Vec<Item> = vec![Item::Value(self)];

        while let Some(item) = stack.pop() {
            match item {
                Item::MapKey(key) => {
                    enc.push_map_k(key)?;
                }
                Item::Value(value) => match value {
                    Value::Null => enc.push_null()?,
                    Value::Bool(v) => enc.push_bool(*v)?,
                    Value::Address(address) => enc.push_address(&address)?,
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
}

impl<W: Writer> Encode<W> for bool {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_bool(*self)
    }
}

macro_rules! impl_encode_signed {
    ($($ty:ty),*) => {
        $(
            impl<W: Writer> Encode<W> for $ty {
                type Error = W::Error;
                fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
                    enc.push_i64(*self as i64)
                }
            }
        )*
    };
}

macro_rules! impl_encode_unsigned {
    ($($ty:ty),*) => {
        $(
            impl<W: Writer> Encode<W> for $ty {
                type Error = W::Error;
                fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
                    enc.push_u64(*self as u64)
                }
            }
        )*
    };
}

macro_rules! impl_encode_unsigned_atomic {
    ($($ty:ty : $as_ty:ty : $push:ident),*) => {
        $(
            impl<W: Writer> Encode<W> for $ty {
                type Error = W::Error;
                fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
                    enc.$push(self.load(std::sync::atomic::Ordering::SeqCst) as $as_ty)
                }
            }
        )*
    };
}

impl_encode_signed!(i8, i16, i32, i64);
impl_encode_unsigned!(u8, u16, u32, u64, usize);
impl_encode_unsigned_atomic!(
    std::sync::atomic::AtomicU8 : u64 : push_u64,
    std::sync::atomic::AtomicU16 : u64 : push_u64,
    std::sync::atomic::AtomicU32 : u64 : push_u64,
    std::sync::atomic::AtomicU64 : u64 : push_u64,
    std::sync::atomic::AtomicUsize : u64 : push_u64,
    std::sync::atomic::AtomicI8 : i64 : push_i64,
    std::sync::atomic::AtomicI16 : i64 : push_i64,
    std::sync::atomic::AtomicI32 : i64 : push_i64,
    std::sync::atomic::AtomicI64 : i64 : push_i64
);

impl<W: Writer> Encode<W> for str {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_str(self)
    }
}

impl<W: Writer> Encode<W> for String {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_str(self)
    }
}

impl<W: Writer> Encode<W> for [u8] {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_bytes(self)
    }
}

impl<W: Writer> Encode<W> for bytes::Bytes {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_bytes(self)
    }
}

impl<W: Writer, T: Encode<W, Error = W::Error>> Encode<W> for Vec<T> {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.start_array(self.len() as u64)?;
        for item in self {
            item.encode(enc)?;
        }
        Ok(())
    }
}

impl<W: Writer, T: Encode<W, Error = W::Error>> Encode<W>
    for std::collections::BTreeMap<String, T>
{
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.start_map(self.len() as u64)?;
        for (k, v) in self {
            enc.push_map_k(k)?;
            v.encode(enc)?;
        }
        Ok(())
    }
}

impl<W: Writer, T: Encode<W, Error = W::Error>> Encode<W> for Option<T> {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        match self {
            Some(value) => value.encode(enc),
            None => enc.push_null(),
        }
    }
}

impl<W: Writer> Encode<W> for num_bigint::BigInt {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_bigint(self)
    }
}

impl<W: Writer> Encode<W> for Address {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_address(self)
    }
}

impl<W: Writer> Encode<W> for () {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        enc.push_null()
    }
}

impl<W: Writer, T: Encode<W>> Encode<W> for Box<T> {
    type Error = <T as Encode<W>>::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        self.as_ref().encode(enc)
    }
}

impl<W: Writer, T: Encode<W>> Encode<W> for Arc<T> {
    type Error = <T as Encode<W>>::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        self.as_ref().encode(enc)
    }
}

impl<W: Writer> Encode<W> for primitive_types::U256 {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> Result<(), Self::Error> {
        let bytes = self.to_little_endian();
        let big = num_bigint::BigInt::from_bytes_le(num_bigint::Sign::Plus, &bytes);
        enc.push_bigint(&big)
    }
}
