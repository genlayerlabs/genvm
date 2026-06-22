//! A 32-byte hash (runner content hash, `custom:` hash, …) with GVM32 formatting.

/// A 32-byte hash value. Formats (and parses) as GVM32 / Crockford Base32 (see
/// [`genlayer_sdk::gvm32`]).
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Bytes32Hash(pub [u8; 32]);

impl Bytes32Hash {
    pub const ZERO: Self = Self([0; 32]);

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn raw(&self) -> [u8; 32] {
        self.0
    }

    /// The GVM32 (Crockford Base32) textual form.
    pub fn to_gvm32(&self) -> String {
        genlayer_sdk::gvm32::encode(&self.0)
    }

    /// Parses a hash from its GVM32 form. Returns `None` for an invalid encoding
    /// or one that does not decode to exactly 32 bytes.
    pub fn from_gvm32(s: &str) -> Option<Self> {
        genlayer_sdk::gvm32::decode(s)?.try_into().ok().map(Self)
    }
}

impl From<[u8; 32]> for Bytes32Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Display for Bytes32Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_gvm32())
    }
}

impl std::fmt::Debug for Bytes32Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("hash#{}", self.to_gvm32()))
    }
}
