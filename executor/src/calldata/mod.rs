mod bin;
mod de;
mod error;
mod se;
mod types;

pub use bin::{decode, encode};
pub use error::*;
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
    match full_type_name {
        "num_bigint::bigint::BigInt" => {
            let as_ptr = std::ptr::from_ref(value) as *mut num_bigint::BigInt; // should be const but non-null...
            Ok(Value::Number(
                unsafe { std::ptr::NonNull::new_unchecked(as_ptr).as_ref() }.clone(),
            ))
        }
        "genvm::calldata::types::Value" => {
            let as_ptr = std::ptr::from_ref(value) as *mut Value; // should be const but non-null...
            Ok(unsafe { std::ptr::NonNull::new_unchecked(as_ptr).as_ref() }.clone())
        }
        "genvm::calldata::types::Address" => {
            let as_ptr = std::ptr::from_ref(value) as *const Address;
            Ok(Value::Address(unsafe { as_ptr.read() }))
        }
        _ => value.serialize(se::Serializer),
    }
}
