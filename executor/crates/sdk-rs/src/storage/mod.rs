//! Storage types for GenLayer smart contracts.

pub mod array;
pub mod core;
pub mod record;
pub mod tree_map;

pub use self::array::*;
pub use self::core::*;
pub use self::tree_map::*;

use crate::calldata::Address;

// Field order defines slot offsets; must match `root_offsets` in the public ABI
// and `Root` in runners/genlayer-py-std/.../storage/root.py:
// MAJOR=0, CONTRACT=1, CODE=2, LOCKED_SLOTS=3, UPGRADERS=4, CODE_SLOT=5.
crate::record!(Root[T] {
    major: u8,
    contract_instance: Indirection<T>,
    code: Indirection<VLA<u8>>,
    locked_slots: Indirection<VLA<primitive_types::U256>>,
    upgraders: Indirection<VLA<Address>>,
    code_slot: primitive_types::U256,
});

impl<T> Root<T> {
    pub const SLOT: Slot = Slot([0u8; 32]);
}

impl<T: StorageType> Root<T> {
    pub fn get() -> Self {
        <Self as StorageType>::handle_at(Self::SLOT, 0)
    }

    /// Resolve the handle to the contract code.
    ///
    /// If `code_slot` is zero (the default), the code is read from the default
    /// `code` indirection; otherwise it is read from the slot pointed to by
    /// `code_slot` (see `chain:` runner ids). Mirrors `Root.get_resolved_code`
    /// in the Python runner.
    pub fn get_resolved_code(&self) -> StorageVLA<u8> {
        let code_slot = self.code_slot().get();
        if code_slot.is_zero() {
            self.code().get()
        } else {
            VLA::<u8>::handle_at(Slot(code_slot.to_little_endian()), 0)
        }
    }
}
