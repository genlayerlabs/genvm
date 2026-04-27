use std::collections::BTreeMap;

use super::consts::*;

// ── Helpers for external types without Arbitrary ─────────────────────

pub fn arb_bytes(u: &mut arbitrary::Unstructured) -> arbitrary::Result<bytes::Bytes> {
    Ok(bytes::Bytes::from(u.arbitrary::<Vec<u8>>()?))
}

pub fn arb_u256(u: &mut arbitrary::Unstructured) -> arbitrary::Result<primitive_types::U256> {
    Ok(primitive_types::U256::from_little_endian(
        &u.arbitrary::<[u8; 32]>()?,
    ))
}

pub fn arb_vec_bytes(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<bytes::Bytes>> {
    let len = u.int_in_range(0..=4u8)?;
    (0..len).map(|_| arb_bytes(u)).collect()
}

pub fn arb_btreemap_bytes(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<BTreeMap<String, bytes::Bytes>> {
    let len = u.int_in_range(0..=4u8)?;
    (0..len)
        .map(|_| Ok((u.arbitrary::<String>()?, arb_bytes(u)?)))
        .collect()
}

// ── Arbitrary for consts enums ───────────────────────────────────────

macro_rules! impl_arbitrary_via_try_from_u8 {
    ($ty:ty, $max:expr) => {
        impl arbitrary::Arbitrary<'_> for $ty {
            fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
                let v = u.int_in_range(0..=$max)?;
                Self::try_from(v).map_err(|_| arbitrary::Error::IncorrectFormat)
            }
        }
    };
}

impl_arbitrary_via_try_from_u8!(ResultCode, 3u8);
impl_arbitrary_via_try_from_u8!(StorageType, 2u8);
impl_arbitrary_via_try_from_u8!(EntryKind, 2u8);

impl arbitrary::Arbitrary<'_> for SpecialMethod {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        if u.arbitrary::<bool>()? {
            Ok(SpecialMethod::GetSchema)
        } else {
            Ok(SpecialMethod::ErroredMessage)
        }
    }
}
