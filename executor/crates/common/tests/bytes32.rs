use genvm_common::Bytes32Hash;

#[test]
fn gvm32_round_trip() {
    let bytes = [0xabu8; 32];
    let h = Bytes32Hash::from_bytes(bytes);

    let s = h.to_gvm32();
    assert_eq!(s.len(), 52); // ceil(256 / 5)
    assert_eq!(Bytes32Hash::from_gvm32(&s), Some(h));
    assert_eq!(h.to_string(), s); // Display == gvm32
    assert_eq!(h.as_bytes(), &bytes);
}

#[test]
fn from_gvm32_rejects_invalid() {
    assert_eq!(Bytes32Hash::from_gvm32("abc"), None); // decodes to < 32 bytes
    assert_eq!(Bytes32Hash::from_gvm32(""), None); // 0 bytes
}
