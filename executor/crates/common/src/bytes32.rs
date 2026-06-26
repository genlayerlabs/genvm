//! A 32-byte hash (runner content hash, `custom:` hash, …) with GVM32 formatting.

/// A 32-byte hash value. Formats (and parses) as GVM32 / Crockford Base32 (see
/// [`genlayer_sdk::gvm32`]).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
    arbitrary::Arbitrary,
)]
#[repr(transparent)]
pub struct Bytes32Hash(
    #[serde(with = "serde_bytes")]
    #[calldata(
        serialize_with = genlayer_calldata::codec::as_bytes::serialize,
        deserialize_with = genlayer_calldata::codec::as_bytes::deserialize,
    )]
    pub [u8; 32],
);

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

    /// Parses a hash from its GVM32 form.
    ///
    /// Fails with [`FromGvm32Error::InvalidEncoding`] for a malformed encoding or
    /// [`FromGvm32Error::WrongLength`] for one that does not decode to exactly 32
    /// bytes.
    pub fn from_gvm32(s: &str) -> Result<Self, FromGvm32Error> {
        let bytes = genlayer_sdk::gvm32::decode(s).map_err(FromGvm32Error::InvalidEncoding)?;
        let got = bytes.len();
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| FromGvm32Error::WrongLength { got })?;
        Ok(Self(bytes))
    }
}

/// Error returned by [`Bytes32Hash::from_gvm32`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromGvm32Error {
    /// The string is not valid GVM32.
    InvalidEncoding(genlayer_sdk::gvm32::DecodeError),
    /// Decoded successfully, but not to exactly 32 bytes.
    WrongLength { got: usize },
}

impl std::fmt::Display for FromGvm32Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FromGvm32Error::InvalidEncoding(e) => write!(f, "invalid gvm32 encoding: {e}"),
            FromGvm32Error::WrongLength { got } => {
                write!(f, "expected 32 bytes, got {got}")
            }
        }
    }
}

impl std::error::Error for FromGvm32Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FromGvm32Error::InvalidEncoding(e) => Some(e),
            FromGvm32Error::WrongLength { .. } => None,
        }
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
