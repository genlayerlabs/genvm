use std::collections::BTreeMap;

pub const ADDRESS_SIZE: usize = 20;

pub struct Address(pub [u8; ADDRESS_SIZE]);

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
