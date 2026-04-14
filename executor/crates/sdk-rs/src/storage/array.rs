//! Fixed-size arrays, dynamically-sized arrays, and variable-length arrays.

use super::core::{Slot, StorageType};

// ===== Fixed-size array =====
// Layout: N elements sequentially at offset + i * element_size.

pub struct StorageArray<T: StorageType, const N: usize> {
    pub slot: Slot,
    pub offset: u32,
    _marker: core::marker::PhantomData<T>,
}

impl<T: StorageType, const N: usize> Clone for StorageArray<T, N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: StorageType, const N: usize> Copy for StorageArray<T, N> {}

impl<T: StorageType, const N: usize> StorageArray<T, N> {
    pub const fn len(&self) -> usize {
        N
    }

    pub fn index(&self, idx: usize) -> T::Handle {
        assert!(idx < N, "index {idx} out of bounds for array of length {N}");
        T::handle_at(self.slot, self.offset + idx as u32 * T::SIZE)
    }
}

impl<T: StorageType, const N: usize> StorageType for [T; N] {
    const SIZE: u32 = T::SIZE * N as u32;
    type Handle = StorageArray<T, N>;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageArray {
            slot,
            offset,
            _marker: core::marker::PhantomData,
        }
    }
}

// ===== Dynamically-sized array =====
// Layout: [u32 length] at (slot, offset), elements at slot.indirect(offset).

/// Marker type for a dynamically-sized storage array.
pub struct DynArray<T>(core::marker::PhantomData<T>);

pub struct StorageDynArray<T: StorageType> {
    pub slot: Slot,
    pub offset: u32,
    _marker: core::marker::PhantomData<T>,
}

impl<T: StorageType> Clone for StorageDynArray<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: StorageType> Copy for StorageDynArray<T> {}

impl<T: StorageType> StorageDynArray<T> {
    pub fn len(&self) -> u32 {
        u32::handle_at(self.slot, self.offset).get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_len(&self, len: u32) {
        u32::handle_at(self.slot, self.offset).set(len);
    }

    pub fn index(&self, idx: u32) -> T::Handle {
        assert!(idx < self.len(), "index out of bounds");
        T::handle_at(self.slot.indirect(self.offset), idx * T::SIZE)
    }

    /// Grow by one and return a handle to the new (uninitialized) element.
    pub fn append_slot(&self) -> T::Handle {
        let len = self.len();
        self.set_len(len + 1);
        T::handle_at(self.slot.indirect(self.offset), len * T::SIZE)
    }

    pub fn pop(&self) {
        let len = self.len();
        assert!(len > 0, "can't pop from empty array");
        self.set_len(len - 1);
    }

    pub fn clear(&self) {
        self.set_len(0);
    }
}

impl<T: StorageType> StorageType for DynArray<T> {
    const SIZE: u32 = 4;
    type Handle = StorageDynArray<T>;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageDynArray {
            slot,
            offset,
            _marker: core::marker::PhantomData,
        }
    }
}

// ===== VLA (Variable Length Array) =====
// Layout: [u32 length] then elements inline in the same slot (no indirection).

/// Marker type for a variable-length array with inline storage.
pub struct VLA<T>(core::marker::PhantomData<T>);

pub struct StorageVLA<T: StorageType> {
    pub slot: Slot,
    pub offset: u32,
    _marker: core::marker::PhantomData<T>,
}

impl<T: StorageType> Clone for StorageVLA<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: StorageType> Copy for StorageVLA<T> {}

impl<T: StorageType> StorageVLA<T> {
    pub fn len(&self) -> u32 {
        u32::handle_at(self.slot, self.offset).get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_len(&self, len: u32) {
        u32::handle_at(self.slot, self.offset).set(len);
    }

    pub fn index(&self, idx: u32) -> T::Handle {
        assert!(idx < self.len(), "index out of bounds");
        T::handle_at(self.slot, self.offset + 4 + idx * T::SIZE)
    }

    /// Grow by one and return a handle to the new (uninitialized) element.
    pub fn append_slot(&self) -> T::Handle {
        let len = self.len();
        self.set_len(len + 1);
        T::handle_at(self.slot, self.offset + 4 + len * T::SIZE)
    }

    pub fn truncate(&self, to: u32) {
        assert!(to <= self.len(), "truncate target exceeds current length");
        self.set_len(to);
    }
}

impl<T: StorageType> StorageType for VLA<T> {
    const SIZE: u32 = u32::MAX;
    type Handle = StorageVLA<T>;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageVLA {
            slot,
            offset,
            _marker: core::marker::PhantomData,
        }
    }
}
