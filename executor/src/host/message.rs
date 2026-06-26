use arbitrary::Arbitrary;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use sha3::Digest;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Copy, Arbitrary)]
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
#[derive(
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Clone,
    Hash,
    Copy,
    PartialOrd,
    Ord,
    Arbitrary,
)]
#[repr(C)]
pub struct SlotID(#[serde_as(as = "Base64")] pub [u8; 32]);

impl<W: genvm_common::calldata::Writer> genvm_common::calldata::codec::Encode<W> for SlotID {
    type Error = W::Error;

    fn encode(
        &self,
        enc: &mut genvm_common::calldata::Encoder<W>,
    ) -> std::result::Result<(), Self::Error> {
        enc.push_bytes(&self.0)
    }
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

    pub fn from_bytes(data: [u8; 32]) -> Self {
        SlotID(data)
    }
}

impl From<[u8; 32]> for SlotID {
    fn from(value: [u8; 32]) -> Self {
        SlotID(value)
    }
}

impl From<&[u8; 32]> for SlotID {
    fn from(value: &[u8; 32]) -> Self {
        SlotID(*value)
    }
}

impl std::fmt::Display for SlotID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&genlayer_sdk::gvm32::encode(&self.0))
    }
}
