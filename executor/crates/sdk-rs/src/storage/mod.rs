//! Storage types for GenLayer smart contracts.

pub mod array;
pub mod core;
pub mod record;
pub mod tree_map;

pub use self::array::*;
pub use self::core::*;
pub use self::tree_map::*;

use crate::calldata::Address;

crate::record!(Root[T] {
    contract_instance: Indirection<T>,
    code: Indirection<VLA<u8>>,
    locked_slots: Indirection<VLA<primitive_types::U256>>,
    upgraders: Indirection<VLA<Address>>,
    major: u8,
});

impl<T> Root<T> {
    pub const SLOT: Slot = Slot([0u8; 32]);
}

impl<T: StorageType> Root<T> {
    pub fn get() -> Self {
        <Self as StorageType>::handle_at(Self::SLOT, 0)
    }
}
