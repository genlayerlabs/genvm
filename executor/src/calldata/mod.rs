mod error;
mod bin;
mod de;
mod se;
mod types;

pub use error::*;
pub use bin::{decode, encode};
pub use types::*;

pub fn from_value<T>(value: Value) -> core::result::Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}

pub fn to_value<T>(value: &T) -> Result<Value, Error>
where
    T: ?Sized + serde::ser::Serialize,
{
    let full_type_name = std::any::type_name::<T>();
    if full_type_name == "num_bigint::bigint::BigInt" {
        let as_ptr = std::ptr::from_ref(value) as *const num_bigint::BigInt;
        Ok(Value::Number(unsafe { as_ptr.read() }))
    } else if full_type_name == "genvm::calldata::types::Address" {
        let as_ptr = std::ptr::from_ref(value) as *const Address;
        Ok(Value::Address(unsafe { as_ptr.read() }))
    } else {
        value.serialize(se::Serializer)
    }
}
