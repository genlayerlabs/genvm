//! Core storage primitives: Slot, StorageType trait, and built-in scalar types.

use crate::abi::wasi;

// ===== Slot =====

/// A 32-byte addressed region of contract storage.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Slot(pub [u8; 32]);

impl Slot {
    pub fn read(&self, offset: u32, buf: &mut [u8]) {
        wasi::storage_read(&self.0, offset, buf).expect("storage read failed");
    }

    pub fn write(&self, offset: u32, buf: &[u8]) {
        wasi::storage_write(&self.0, offset as i32, buf).expect("storage write failed");
    }

    /// Derive a child slot via `sha3_256(slot_id || offset_le_bytes)`.
    pub fn indirect(&self, offset: u32) -> Slot {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(self.0);
        hasher.update(offset.to_le_bytes());
        Slot(hasher.finalize().into())
    }
}

// ===== StorageType trait =====

/// Maps a logical value type to its storage handle.
///
/// `SIZE` is the number of bytes occupied in the parent slot.
/// `Handle` is the pointer-like struct used to read/write the value.
pub trait StorageType {
    const SIZE: u32;
    type Handle;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle;
}

// ===== Integer types =====

macro_rules! impl_int_storage {
    ($($rust_ty:ty => $handle_name:ident),* $(,)?) => { $(
        #[derive(Clone, Copy)]
        pub struct $handle_name {
            pub slot: Slot,
            pub offset: u32,
        }

        impl $handle_name {
            pub fn get(&self) -> $rust_ty {
                let mut buf = [0u8; core::mem::size_of::<$rust_ty>()];
                self.slot.read(self.offset, &mut buf);
                <$rust_ty>::from_le_bytes(buf)
            }

            pub fn set(&self, val: $rust_ty) {
                self.slot.write(self.offset, &val.to_le_bytes());
            }
        }

        impl StorageType for $rust_ty {
            const SIZE: u32 = core::mem::size_of::<$rust_ty>() as u32;
            type Handle = $handle_name;
            fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
                $handle_name { slot, offset }
            }
        }
    )* };
}

impl_int_storage! {
    u8  => StorageU8,
    u16 => StorageU16,
    u32 => StorageU32,
    u64 => StorageU64,
    u128 => StorageU128,
    i8  => StorageI8,
    i16 => StorageI16,
    i32 => StorageI32,
    i64 => StorageI64,
    i128 => StorageI128,
}

// ===== Float types =====

macro_rules! impl_float_storage {
    ($($rust_ty:ty => $handle_name:ident),* $(,)?) => { $(
        #[derive(Clone, Copy)]
        pub struct $handle_name {
            pub slot: Slot,
            pub offset: u32,
        }

        impl $handle_name {
            pub fn get(&self) -> $rust_ty {
                let mut buf = [0u8; core::mem::size_of::<$rust_ty>()];
                self.slot.read(self.offset, &mut buf);
                <$rust_ty>::from_le_bytes(buf)
            }

            pub fn set(&self, val: $rust_ty) {
                self.slot.write(self.offset, &val.to_le_bytes());
            }
        }

        impl StorageType for $rust_ty {
            const SIZE: u32 = core::mem::size_of::<$rust_ty>() as u32;
            type Handle = $handle_name;
            fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
                $handle_name { slot, offset }
            }
        }
    )* };
}

impl_float_storage! {
    f32 => StorageF32,
    f64 => StorageF64,
}

// ===== Bool =====

#[derive(Clone, Copy)]
pub struct StorageBool {
    pub slot: Slot,
    pub offset: u32,
}

impl StorageBool {
    pub fn get(&self) -> bool {
        let mut buf = [0u8; 1];
        self.slot.read(self.offset, &mut buf);
        buf[0] != 0
    }

    pub fn set(&self, val: bool) {
        self.slot.write(self.offset, &[val as u8]);
    }
}

impl StorageType for bool {
    const SIZE: u32 = 1;
    type Handle = StorageBool;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageBool { slot, offset }
    }
}

// ===== String =====
// Layout: [u32 length] at (slot, offset), UTF-8 data at slot.indirect(offset).

#[derive(Clone, Copy)]
pub struct StorageStr {
    pub slot: Slot,
    pub offset: u32,
}

impl StorageStr {
    pub fn len(&self) -> u32 {
        u32::handle_at(self.slot, self.offset).get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_len(&self, len: u32) {
        u32::handle_at(self.slot, self.offset).set(len);
    }

    pub fn load(&self) -> String {
        let len = self.len() as usize;
        if len == 0 {
            return String::new();
        }
        let indirect = self.slot.indirect(self.offset);
        let mut buf = vec![0u8; len];
        indirect.read(0, &mut buf);
        String::from_utf8(buf).expect("invalid utf-8 in storage string")
    }

    pub fn store(&self, val: &str) {
        let bytes = val.as_bytes();
        self.set_len(bytes.len() as u32);
        if !bytes.is_empty() {
            self.slot.indirect(self.offset).write(0, bytes);
        }
    }

    pub fn slice(&self, start: u32, end: u32) -> String {
        assert!(start <= end && end <= self.len());
        let len = (end - start) as usize;
        if len == 0 {
            return String::new();
        }
        let indirect = self.slot.indirect(self.offset);
        let mut buf = vec![0u8; len];
        indirect.read(start, &mut buf);
        String::from_utf8(buf).expect("invalid utf-8 in storage string slice")
    }

    pub fn index(&self, idx: u32) -> u8 {
        assert!(idx < self.len(), "index out of bounds");
        let mut buf = [0u8; 1];
        self.slot.indirect(self.offset).read(idx, &mut buf);
        buf[0]
    }
}

impl StorageType for String {
    const SIZE: u32 = 4;
    type Handle = StorageStr;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageStr { slot, offset }
    }
}

// ===== Bytes =====
// Layout: [u32 length] at (slot, offset), raw data at slot.indirect(offset).

#[derive(Clone, Copy)]
pub struct StorageBytes {
    pub slot: Slot,
    pub offset: u32,
}

impl StorageBytes {
    pub fn len(&self) -> u32 {
        u32::handle_at(self.slot, self.offset).get()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_len(&self, len: u32) {
        u32::handle_at(self.slot, self.offset).set(len);
    }

    pub fn load(&self) -> Vec<u8> {
        let len = self.len() as usize;
        if len == 0 {
            return Vec::new();
        }
        let indirect = self.slot.indirect(self.offset);
        let mut buf = vec![0u8; len];
        indirect.read(0, &mut buf);
        buf
    }

    pub fn store(&self, val: &[u8]) {
        self.set_len(val.len() as u32);
        if !val.is_empty() {
            self.slot.indirect(self.offset).write(0, val);
        }
    }

    pub fn slice(&self, start: u32, end: u32) -> Vec<u8> {
        assert!(start <= end && end <= self.len());
        let len = (end - start) as usize;
        if len == 0 {
            return Vec::new();
        }
        let indirect = self.slot.indirect(self.offset);
        let mut buf = vec![0u8; len];
        indirect.read(start, &mut buf);
        buf
    }

    pub fn index(&self, idx: u32) -> u8 {
        assert!(idx < self.len(), "index out of bounds");
        let mut buf = [0u8; 1];
        self.slot.indirect(self.offset).read(idx, &mut buf);
        buf[0]
    }
}

impl StorageType for Vec<u8> {
    const SIZE: u32 = 4;
    type Handle = StorageBytes;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageBytes { slot, offset }
    }
}

// ===== Unit (None) =====

impl StorageType for () {
    const SIZE: u32 = 0;
    type Handle = ();
    fn handle_at(_slot: Slot, _offset: u32) -> Self::Handle {}
}

// ===== Address =====

#[derive(Clone, Copy)]
pub struct StorageAddress {
    pub slot: Slot,
    pub offset: u32,
}

impl StorageAddress {
    pub fn get(&self) -> crate::calldata::Address {
        let mut buf = [0u8; 20];
        self.slot.read(self.offset, &mut buf);
        crate::calldata::Address::from(buf)
    }

    pub fn set(&self, val: crate::calldata::Address) {
        self.slot.write(self.offset, &val.raw());
    }
}

impl StorageType for crate::calldata::Address {
    const SIZE: u32 = 20;
    type Handle = StorageAddress;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageAddress { slot, offset }
    }
}

// ===== U256 =====

#[derive(Clone, Copy)]
pub struct StorageU256 {
    pub slot: Slot,
    pub offset: u32,
}

impl StorageU256 {
    pub fn get(&self) -> primitive_types::U256 {
        let mut buf = [0u8; 32];
        self.slot.read(self.offset, &mut buf);
        primitive_types::U256::from_little_endian(&buf)
    }

    pub fn set(&self, val: primitive_types::U256) {
        self.slot.write(self.offset, &val.to_little_endian());
    }
}

impl StorageType for primitive_types::U256 {
    const SIZE: u32 = 32;
    type Handle = StorageU256;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageU256 { slot, offset }
    }
}

// ===== Indirection =====
// Data lives at a derived slot. Occupies 1 byte in the parent to prevent collisions.

/// Marker type for indirected storage.
pub struct Indirection<T>(core::marker::PhantomData<T>);

pub struct StorageIndirection<T: StorageType> {
    pub slot: Slot,
    pub offset: u32,
    _marker: core::marker::PhantomData<T>,
}

impl<T: StorageType> Clone for StorageIndirection<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: StorageType> Copy for StorageIndirection<T> {}

impl<T: StorageType> StorageIndirection<T> {
    pub fn get(&self) -> T::Handle {
        T::handle_at(self.slot.indirect(self.offset), 0)
    }
}

impl<T: StorageType> StorageType for Indirection<T> {
    const SIZE: u32 = 1;
    type Handle = StorageIndirection<T>;
    fn handle_at(slot: Slot, offset: u32) -> Self::Handle {
        StorageIndirection {
            slot,
            offset,
            _marker: core::marker::PhantomData,
        }
    }
}
