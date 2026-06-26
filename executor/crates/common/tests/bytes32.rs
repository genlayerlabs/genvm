use genlayer_sdk::gvm32::DecodeError;
use genvm_common::bytes32::FromGvm32Error;
use genvm_common::Bytes32Hash;

#[test]
fn gvm32_round_trip() {
    let bytes = [0xabu8; 32];
    let h = Bytes32Hash::from_bytes(bytes);

    let s = h.to_gvm32();
    assert_eq!(s.len(), 52); // ceil(256 / 5)
    assert_eq!(Bytes32Hash::from_gvm32(&s), Ok(h));
    assert_eq!(h.to_string(), s); // Display == gvm32
    assert_eq!(h.as_bytes(), &bytes);
}

#[test]
fn from_gvm32_rejects_invalid() {
    // valid gvm32, but decodes to fewer than 32 bytes
    assert_eq!(
        Bytes32Hash::from_gvm32("00"), // decodes to 1 byte
        Err(FromGvm32Error::WrongLength { got: 1 })
    );
    assert_eq!(
        Bytes32Hash::from_gvm32(""), // decodes to 0 bytes
        Err(FromGvm32Error::WrongLength { got: 0 })
    );
    // '!' is not in the alphabet
    assert_eq!(
        Bytes32Hash::from_gvm32("!"),
        Err(FromGvm32Error::InvalidEncoding(DecodeError::InvalidChar(
            '!'
        )))
    );
    // valid alphabet, but non-zero trailing padding bits
    assert_eq!(
        Bytes32Hash::from_gvm32("abc"),
        Err(FromGvm32Error::InvalidEncoding(
            DecodeError::NonZeroPadding {
                bits: 7,
                value: 108
            }
        ))
    );
}
