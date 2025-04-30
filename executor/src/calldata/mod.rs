mod error;
mod parse;
mod sd;
mod types;

pub use error::*;
pub use parse::*;
pub use types::*;

pub fn from_value<T>(value: Value) -> core::result::Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}
