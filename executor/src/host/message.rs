use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use sha3::Digest;
use std::sync::Arc;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Copy)]
pub struct AccountAddress(#[serde_as(as = "Base64")] pub [u8; 20]);

impl AccountAddress {
    pub fn raw(&self) -> [u8; 20] {
        let AccountAddress(r) = self;
        *r
    }

    pub fn zero() -> Self {
        Self([0; 20])
    }

    pub const fn len() -> usize {
        20
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Copy, PartialOrd, Ord)]
#[repr(C)]
pub struct SlotID(#[serde_as(as = "Base64")] pub [u8; 32]);

pub mod root_offsets {
    pub const CODE: u32 = 1;
    pub const LOCKED_SLOTS: u32 = 2;
    pub const UPGRADERS: u32 = 3;
}

impl SlotID {
    pub const ZERO: SlotID = SlotID([0; 32]);
    pub const SIZE: u32 = 32;

    pub fn raw(&self) -> [u8; 32] {
        let SlotID(r) = self;
        *r
    }

    pub fn zero() -> Self {
        Self([0; 32])
    }

    pub const fn len() -> usize {
        32
    }

    pub fn indirection(&self, off: u32) -> SlotID {
        let mut digest = sha3::Sha3_256::new();
        digest.update(self.0);
        digest.update(off.to_le_bytes());

        let mut ret = Self::ZERO;
        ret.0.copy_from_slice(digest.finalize().as_slice());
        ret
    }
}

fn default_datetime() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2024-11-26T06:42:42.424242Z")
        .unwrap()
        .to_utc()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageData {
    pub contract_address: AccountAddress,
    pub sender_address: AccountAddress,
    pub origin_address: AccountAddress,
    pub chain_id: Arc<str>,
    pub value: Option<u64>,
    pub is_init: bool,
    #[serde(default = "default_datetime")]
    pub datetime: chrono::DateTime<chrono::Utc>,
}
