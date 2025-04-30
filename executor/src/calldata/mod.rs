mod sd;
mod types;
mod error;
mod parse;

pub use types::*;
pub use error::*;
pub use parse::*;

pub fn from_value<T>(value: Value) -> core::result::Result<T, Error>
where T: serde::de::DeserializeOwned
{
    T::deserialize(value)
}
